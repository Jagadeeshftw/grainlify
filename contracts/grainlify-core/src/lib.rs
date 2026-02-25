//! # Grainlify Contract Upgrade System
//!
//! A minimal, secure contract upgrade pattern for Soroban smart contracts.
//! This contract implements admin-controlled WASM upgrades with version tracking.
//!
//! ## Overview
//!
//! The Grainlify contract provides a foundational upgrade mechanism that allows
//! authorized administrators to update contract logic while maintaining state
//! persistence. This is essential for bug fixes, feature additions, and security
//! patches in production environments.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Contract Upgrade Architecture                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                              │
//! │  ┌──────────────┐                                           │
//! │  │    Admin     │                                           │
//! │  └──────┬───────┘                                           │
//! │         │                                                    │
//! │         │ 1. Compile new WASM                               │
//! │         │ 2. Upload to Stellar                              │
//! │         │ 3. Get WASM hash                                  │
//! │         │                                                    │
//! │         ▼                                                    │
//! │  ┌──────────────────┐                                       │
//! │  │  upgrade(hash)   │────────┐                              │
//! │  └──────────────────┘        │                              │
//! │         │                     │                              │
//! │         │ require_auth()      │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         │              │   Verify    │                       │
//! │         │              │   Admin     │                       │
//! │         │              └──────┬──────┘                       │
//! │         │                     │                              │
//! │         │                     ▼                              │
//! │         │              ┌─────────────┐                       │
//! │         └─────────────>│   Update    │                       │
//! │                        │   WASM      │                       │
//! │                        └──────┬──────┘                       │
//! │                               │                              │
//! │                               ▼                              │
//! │                        ┌─────────────┐                       │
//! │                        │ New Version │                       │
//! │                        │  (Optional) │                       │
//! │                        └─────────────┘                       │
//! │                                                              │
//! │  Storage:                                                    │
//! │  ┌────────────────────────────────────┐                     │
//! │  │ Admin: Address                     │                     │
//! │  │ Version: u32                       │                     │
//! │  └────────────────────────────────────┘                     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Security Model
//!
//! ### Trust Assumptions
//! - **Admin**: Highly trusted entity with upgrade authority
//! - **WASM Code**: New code must be audited before deployment
//! - **State Preservation**: Upgrades preserve existing contract state
//!
//! ### Security Features
//! 1. **Single Admin**: Only one authorized address can upgrade
//! 2. **Authorization Check**: Every upgrade requires admin signature
//! 3. **Version Tracking**: Auditable upgrade history
//! 4. **State Preservation**: Instance storage persists across upgrades
//! 5. **Immutable After Init**: Admin cannot be changed after initialization
//!
//! ### Security Considerations
//! - Admin key should be secured with hardware wallet or multi-sig
//! - New WASM should be audited before upgrade
//! - Consider implementing timelock for high-value contracts
//! - Version updates should follow semantic versioning
//! - Test upgrades on testnet before mainnet deployment
//!
//! ## Upgrade Process
//!
//! ```rust
//! // 1. Initialize contract (one-time)
//! let admin = Address::from_string("GADMIN...");
//! contract.init(&admin);
//!
//! // 2. Develop and test new version locally
//! // ... make changes to contract code ...
//!
//! // 3. Build new WASM
//! // $ cargo build --release --target wasm32-unknown-unknown
//!
//! // 4. Upload WASM to Stellar and get hash
//! // $ stellar contract install --wasm target/wasm32-unknown-unknown/release/contract.wasm
//! // Returns: hash (e.g., "abc123...")
//!
//! // 5. Perform upgrade
//! let wasm_hash = BytesN::from_array(&env, &[0xab, 0xcd, ...]);
//! contract.upgrade(&wasm_hash);
//!
//! // 6. (Optional) Update version number
//! contract.set_version(&2);
//!
//! // 7. Verify upgrade
//! let version = contract.get_version();
//! assert_eq!(version, 2);
//! ```
//!
//! ## State Migration
//!
//! When upgrading contracts that require state migration:
//!
//! ```rust
//! // In new WASM version, add migration function:
//! pub fn migrate(env: Env) {
//!     let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
//!     admin.require_auth();
//!     
//!     // Perform state migration
//!     // Example: Convert old data format to new format
//!     let old_version = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
//!     
//!     if old_version < 2 {
//!         // Migrate from v1 to v2
//!         migrate_v1_to_v2(&env);
//!     }
//!     
//!     // Update version
//!     env.storage().instance().set(&DataKey::Version, &2u32);
//! }
//! ```
//!
//! ## Best Practices
//!
//! 1. **Version Numbering**: Use semantic versioning (MAJOR.MINOR.PATCH)
//! 2. **Testing**: Always test upgrades on testnet first
//! 3. **Auditing**: Audit new code before mainnet deployment
//! 4. **Documentation**: Document breaking changes between versions
//! 5. **Rollback Plan**: Keep previous WASM hash for emergency rollback
//! 6. **Admin Security**: Use multi-sig or timelock for production
//! 7. **State Validation**: Verify state integrity after upgrade
//!
//! ## Common Pitfalls
//!
//! - ❌ Not testing upgrades on testnet
//! - ❌ Losing admin private key
//! - ❌ Breaking state compatibility between versions
//! - ❌ Not documenting migration steps
//! - ❌ Upgrading without proper testing
//! - ❌ Not having a rollback plan





#![no_std]

mod multisig;
use multisig::{MultiSig, MultiSigConfig};
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, Symbol, Vec, String,
};
pub mod asset;
pub mod nonce;

mod governance;
mod multisig;
pub use governance::{
    Error as GovError, GovernanceConfig, Proposal, ProposalStatus, Vote, VoteType, VotingScheme,
};
// use multisig::MultiSig; // Currently unused - uncomment when needed
#[cfg(feature = "contract")]
use soroban_sdk::{contract, contractimpl};
// use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env, String, Symbol, Vec}; // Some imports currently unused
use soroban_sdk::{contracttype, symbol_short, BytesN, Env, String, Symbol};

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
// Contract Definition
// ============================================================================

#[cfg(feature = "contract")]
#[contract]
pub struct GrainlifyContract;

// ============================================================================
// Data Structures
// ============================================================================

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the administrator address (set once at initialization)
/// * `Version` - Stores the current contract version number
/// * `ChainId` - Stores the chain identifier for cross-network protection
/// * `NetworkId` - Stores the network identifier for environment-specific behavior
///
/// # Storage Type
/// Instance storage - Persists across contract upgrades
///
/// # Security Note
/// These keys use instance storage to ensure data survives WASM upgrades.
/// The admin address is immutable after initialization.
#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Administrator address with upgrade authority
    Admin,

    /// Current version number (increments with upgrades)
    Version,

    // NEW: store wasm hash per proposal
    UpgradeProposal(u64),
    
    /// Migration state tracking - prevents double migration
    MigrationState,
    
    /// Previous version before migration (for rollback support)
    PreviousVersion,

    /// Chain identifier (e.g., "stellar", "ethereum") for cross-network protection
    ChainId,

    /// Network identifier (e.g., "mainnet", "testnet", "futurenet") for environment-specific behavior
    NetworkId,
    /// Configuration snapshot data by snapshot id
    ConfigSnapshot(u64),

    /// Ordered list of retained snapshot ids
    SnapshotIndex,

    /// Monotonic snapshot id counter
    SnapshotCounter,
}

// ============================================================================
// Constants
// ============================================================================

/// Current contract version.
///
/// This constant should be incremented with each contract upgrade:
/// - MAJOR version: Breaking changes
/// - MINOR version: New features (backward compatible)
/// - PATCH version: Bug fixes
///
/// # Version History
/// - v1: Initial release with basic upgrade functionality
/// - v2: Added state migration system
///
/// # Usage
/// Set during initialization and can be updated via `set_version()`.
#[cfg(feature = "contract")]
const VERSION: u32 = 2;
const CONFIG_SNAPSHOT_LIMIT: u32 = 20;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreConfigSnapshot {
    pub id: u64,
    pub timestamp: u64,
    pub admin: Option<Address>,
    pub version: u32,
    pub previous_version: Option<u32>,
    pub multisig_config: Option<MultiSigConfig>,
}

// ============================================================================
// Migration System
// ============================================================================

/// Migration state tracking to prevent double migration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationState {
    /// Version that was migrated from
    pub from_version: u32,
    /// Version that was migrated to
    pub to_version: u32,
    /// Timestamp when migration completed
    pub migrated_at: u64,
    /// Migration hash for verification
    pub migration_hash: BytesN<32>,
}

/// Migration event data
#[contracttype]
#[derive(Clone, Debug)]
pub struct MigrationEvent {
    pub from_version: u32,
    pub to_version: u32,
    pub timestamp: u64,
    pub migration_hash: BytesN<32>,
    pub success: bool,
    pub error_message: Option<String>,
}

// ============================================================================
// Contract Implementation
// ============================================================================


    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initializes the contract with an admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    ///
    /// # Panics
    /// * If contract is already initialized
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage
    /// - Sets initial Version number
    ///
    /// # Security Considerations
    /// - Can only be called once (prevents admin takeover)
    /// - Admin address is immutable after initialization
    /// - Admin should be a secure address (hardware wallet/multi-sig)
    /// - No authorization required for initialization (first-caller pattern)
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, Env};
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    ///
    /// // Initialize contract
    /// contract.init(&env, &admin);
    ///
    /// // Subsequent init attempts will panic
    /// // contract.init(&env, &another_admin); // ❌ Panics!
    /// ```
    ///
    /// # Gas Cost
    /// Low - Two storage writes
    ///
    /// # Production Deployment
    /// ```bash
    /// # Deploy contract
    /// stellar contract deploy \
    ///   --wasm target/wasm32-unknown-unknown/release/grainlify.wasm \
    ///   --source ADMIN_SECRET_KEY
    ///
    /// # Initialize with admin address
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- init \
    ///   --admin GADMIN_ADDRESS
    /// ```
 
#[cfg(feature = "contract")]
#[contractimpl]
impl GrainlifyContract {
    /// Initializes the contract with multisig configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `signers` - List of signer addresses for multisig
    /// * `threshold` - Number of signatures required to execute proposals
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("Already initialized");
        }

        MultiSig::init(&env, signers, threshold);
        env.storage().instance().set(&DataKey::Version, &VERSION);
    }

    /// Initializes the contract with a single admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    pub fn init_admin(env: Env, admin: Address) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect admin immutability
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init"), admin.clone(), false);
            panic!("Already initialized");
        }

        // Store admin address (immutable after this point)
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set initial version
        env.storage().instance().set(&DataKey::Version, &VERSION);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);
    }

    /// Initializes the contract with admin address and network configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    /// * `chain_id` - Chain identifier (e.g., "stellar", "ethereum")
    /// * `network_id` - Network identifier (e.g., "mainnet", "testnet", "futurenet")
    ///
    /// # Security Considerations
    /// - Chain and network IDs are immutable after initialization
    /// - These values prevent cross-network replay attacks
    /// - Should match the actual deployment environment
    pub fn init_with_network(env: Env, admin: Address, chain_id: String, network_id: String) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect immutability
        if env.storage().instance().has(&DataKey::Admin) {
            monitoring::track_operation(&env, symbol_short!("init_net"), admin.clone(), false);
            panic!("Already initialized");
        }

        // Store admin address (immutable after this point)
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set initial version
        env.storage().instance().set(&DataKey::Version, &VERSION);

        // Store chain and network identifiers
        env.storage().instance().set(&DataKey::ChainId, &chain_id);
        env.storage()
            .instance()
            .set(&DataKey::NetworkId, &network_id);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_net"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_net"), duration);
    }



    /// Proposes an upgrade with a new WASM hash (multisig version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposer` - Address proposing the upgrade
    /// * `wasm_hash` - Hash of the new WASM code
    ///
    /// # Returns
    /// * `u64` - The proposal ID
    pub fn propose_upgrade(
        env: Env,
        proposer: Address,
        wasm_hash: BytesN<32>,
    ) -> u64 {
        let proposal_id = MultiSig::propose(&env, proposer);

        env.storage()
            .instance()
            .set(&DataKey::UpgradeProposal(proposal_id), &wasm_hash);

        proposal_id
    }

    /// Approves an upgrade proposal (multisig version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - The ID of the proposal to approve
    /// * `signer` - Address approving the proposal
    pub fn approve_upgrade(
        env: Env,
        proposal_id: u64,
        signer: Address,
    ) {
        MultiSig::approve(&env, proposal_id, signer);
    }


    /// Upgrades the contract to new WASM code.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    ///
    /// # Authorization
    /// - **CRITICAL**: Only admin can call this function
    /// - Admin must sign the transaction
    ///
    /// # State Changes
    /// - Replaces current contract WASM with new version
    /// - Preserves all instance storage (admin, version, etc.)
    /// - Does NOT automatically update version number (call `set_version` separately)
    ///
    /// # Security Considerations
    /// - **Code Review**: New WASM must be audited before deployment
    /// - **Testing**: Test upgrade on testnet first
    /// - **State Compatibility**: Ensure new code is compatible with existing state
    /// - **Rollback Plan**: Keep previous WASM hash for emergency rollback
    /// - **Version Update**: Call `set_version` after upgrade if needed
    ///
    /// # Workflow
    /// 1. Develop and test new contract version
    /// 2. Build WASM: `cargo build --release --target wasm32-unknown-unknown`
    /// 3. Upload WASM to Stellar network
    /// 4. Get WASM hash from upload response
    /// 5. Call this function with the hash
    /// 6. (Optional) Call `set_version` to update version number
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{BytesN, Env};
    ///
    /// let env = Env::default();
    ///
    /// // Upload new WASM and get hash (done off-chain)
    /// let wasm_hash = BytesN::from_array(
    ///     &env,
    ///     &[0xab, 0xcd, 0xef, ...] // 32 bytes
    /// );
    ///
    /// // Perform upgrade (requires admin authorization)
    /// contract.upgrade(&env, &wasm_hash);
    ///
    /// // Update version number
    /// contract.set_version(&env, &2);
    /// ```
    ///
    /// # Production Upgrade Process
    /// ```bash
    /// # 1. Build new WASM
    /// cargo build --release --target wasm32-unknown-unknown
    ///
    /// # 2. Upload WASM to Stellar
    /// stellar contract install \
    ///   --wasm target/wasm32-unknown-unknown/release/grainlify.wasm \
    ///   --source ADMIN_SECRET_KEY
    /// # Output: WASM_HASH (e.g., abc123...)
    ///
    /// # 3. Upgrade contract
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- upgrade \
    ///   --new_wasm_hash WASM_HASH
    ///
    /// # 4. Update version (optional)
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- set_version \
    ///   --new_version 2
    /// ```
    ///
    /// # Gas Cost
    /// High - WASM code replacement is expensive
    ///
    /// # Emergency Rollback
    /// If new version has issues, rollback to previous WASM:
    /// ```bash
    /// stellar contract invoke \
    ///   --id CONTRACT_ID \
    ///   --source ADMIN_SECRET_KEY \
    ///   -- upgrade \
    ///   --new_wasm_hash PREVIOUS_WASM_HASH
    /// ```
    ///
    /// # Panics
    /// * If admin address is not set (contract not initialized)
    /// * If caller is not the admin

    /// Executes an upgrade proposal that has met the multisig threshold.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `proposal_id` - The ID of the upgrade proposal to execute
    pub fn execute_upgrade(env: Env, proposal_id: u64) {
        if !MultiSig::can_execute(&env, proposal_id) {
            panic!("Threshold not met");
        }

        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UpgradeProposal(proposal_id))
            .expect("Missing upgrade proposal");

        env.deployer().update_current_contract_wasm(wasm_hash);

        MultiSig::mark_executed(&env, proposal_id);
    }

    /// Upgrades the contract to new WASM code (single admin version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Store previous version for potential rollback
        let current_version = env.storage().instance().get(&DataKey::Version).unwrap_or(1);
        env.storage().instance().set(&DataKey::PreviousVersion, &current_version);

        // Perform WASM upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("upgrade"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("upgrade"), duration);
    }


    // ========================================================================
    // Version Management
    // ========================================================================

    /// Retrieves the current contract version number.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `u32` - Current version number (defaults to 0 if not set)
    ///
    /// # Usage
    /// Use this to verify contract version for:
    /// - Client compatibility checks
    /// - Migration decision logic
    /// - Audit trails
    /// - Version-specific behavior
    ///
    /// # Example
    /// ```rust
    /// let version = contract.get_version(&env);
    ///
    /// match version {
    ///     1 => println!("Running v1"),
    ///     2 => println!("Running v2 with new features"),
    ///     _ => println!("Unknown version"),
    /// }
    /// ```
    ///
    /// # Client-Side Usage
    /// ```javascript
    /// // Check contract version before interaction
    /// const version = await contract.get_version();
    ///
    /// if (version < 2) {
    ///     throw new Error("Contract version too old, please upgrade");
    /// }
    /// ```
    ///
    /// # Gas Cost
    /// Very Low - Single storage read
    pub fn get_version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }

    /// Returns the semantic version string (e.g., "1.0.0").
    /// Falls back to mapping known numeric values to semantic strings.
    pub fn get_version_semver_string(env: Env) -> String {
        let raw: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0);
        let s = match raw {
            0 => "0.0.0",
            1 | 10000 => "1.0.0",
            2 | 20000 => "2.0.0",
            10100 => "1.1.0",
            10001 => "1.0.1",
            _ => "unknown",
        };
        String::from_str(&env, s)
    }

    /// Returns the numeric encoded semantic version using policy major*10_000 + minor*100 + patch.
    /// If the stored version is a simple major number (1,2,3...), it will be converted to major*10_000.
    pub fn get_version_numeric_encoded(env: Env) -> u32 {
        let raw: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0);
        if raw >= 10_000 { raw } else { raw.saturating_mul(10_000) }
    }

    /// Ensures the current version meets a minimum required encoded semantic version.
    /// Panics if current version is lower than `min_numeric`.
    pub fn require_min_version(env: Env, min_numeric: u32) {
        let cur = Self::get_version_numeric_encoded(env.clone());
        if cur < min_numeric {
            panic!("Incompatible contract version");
        }
    }


    /// Updates the contract version number.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_version` - New version number to set
    ///
    /// # Authorization
    /// - Only admin can call this function
    /// - Admin must sign the transaction
    ///
    /// # State Changes
    /// - Updates Version in instance storage
    ///
    /// # Usage
    /// Call this function after upgrading contract WASM to reflect
    /// the new version number. This provides an audit trail of upgrades.
    ///
    /// # Version Numbering Strategy
    /// Recommend using semantic versioning encoded as single u32:
    /// - `1` = v1.0.0
    /// - `2` = v2.0.0
    /// - `101` = v1.0.1 (patch)
    /// - `110` = v1.1.0 (minor)
    ///
    /// Or use simple incrementing:
    /// - `1` = First version
    /// - `2` = Second version
    /// - `3` = Third version
    ///
    /// # Example
    /// ```rust
    /// // After upgrading WASM
    /// contract.upgrade(&env, &new_wasm_hash);
    ///
    /// // Update version to reflect the upgrade
    /// contract.set_version(&env, &2);
    ///
    /// // Verify
    /// assert_eq!(contract.get_version(&env), 2);
    /// ```
    ///
    /// # Best Practice
    /// Document version changes:
    /// ```rust
    /// // Version History:
    /// // 1 - Initial release
    /// // 2 - Added feature X, fixed bug Y
    /// // 3 - Performance improvements
    /// contract.set_version(&env, &3);
    /// ```
    ///
    /// # Security Note
    /// This function does NOT perform the actual upgrade.
    /// It only updates the version metadata. Always call
    /// `upgrade()` first, then `set_version()`.
    ///
    /// # Gas Cost
    /// Very Low - Single storage write
    ///
    /// # Panics
    /// * If admin address is not set (contract not initialized)
    /// * If caller is not the admin


    pub fn set_version(env: Env, new_version: u32) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Update version number
        env.storage()
            .instance()
            .set(&DataKey::Version, &new_version);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("set_ver"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("set_ver"), duration);
    }

    /// Retrieves the chain identifier.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Option<String>` - Chain identifier if set, None if not initialized with network config
    ///
    /// # Usage
    /// Use this to verify the chain environment for:
    /// - Cross-network protection
    /// - Replay attack prevention
    /// - Environment-specific behavior
    ///
    /// # Example
    /// ```rust
    /// let chain_id = contract.get_chain_id(&env);
    /// match chain_id.as_str() {
    ///     "stellar" => println!("Running on Stellar network"),
    ///     "ethereum" => println!("Running on Ethereum network"),
    ///     _ => println!("Unknown chain"),
    /// }
    /// ```
    pub fn get_chain_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::ChainId)
    }

    /// Retrieves the network identifier.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `Option<String>` - Network identifier if set, None if not initialized with network config
    ///
    /// # Usage
    /// Use this to verify the network environment for:
    /// - Environment-specific behavior
    /// - Testnet vs mainnet differentiation
    /// - Safe replay protection
    ///
    /// # Example
    /// ```rust
    /// let network_id = contract.get_network_id(&env);
    /// match network_id.as_str() {
    ///     "mainnet" => println!("Running on mainnet - be careful!"),
    ///     "testnet" => println!("Running on testnet - safe for testing"),
    ///     "futurenet" => println!("Running on futurenet - experimental"),
    ///     _ => println!("Unknown network"),
    /// }
    /// ```
    pub fn get_network_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::NetworkId)
    }

    /// Gets both chain and network identifiers as a tuple.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    ///
    /// # Returns
    /// * `(Option<String>, Option<String>)` - Tuple of (chain_id, network_id)
    ///
    /// # Usage
    /// Convenience function to get both identifiers in one call.
    ///
    /// # Example
    /// ```rust
    /// let (chain, network) = contract.get_network_info(&env);
    /// println!("Chain: {:?}, Network: {:?}", chain, network);
    /// ```
    pub fn get_network_info(env: Env) -> (Option<String>, Option<String>) {
        (
            env.storage().instance().get(&DataKey::ChainId),
            env.storage().instance().get(&DataKey::NetworkId),
        )
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

    // ========================================================================
    // State Migration System
    // ========================================================================

    /// Executes state migration from current version to target version.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `target_version` - Version to migrate to
    /// * `migration_hash` - Hash of migration data for verification
    ///
    /// # Authorization
    /// - Only admin can call this function
    /// - Admin must sign the transaction
    ///
    /// # State Changes
    /// - Migrates contract state from current version to target version
    /// - Updates version number
    /// - Records migration state to prevent double migration
    /// - Emits migration event
    ///
    /// # Migration Process
    /// 1. Validates current version and target version
    /// 2. Checks if migration already completed
    /// 3. Executes version-specific migration functions
    /// 4. Updates version number
    /// 5. Records migration state
    /// 6. Emits migration event
    ///
    /// # Example
    /// ```rust
    /// // After upgrading WASM to v2
    /// contract.upgrade(&env, &new_wasm_hash);
    ///
    /// // Migrate state from v1 to v2
    /// let migration_hash = BytesN::from_array(&env, &[...]);
    /// contract.migrate(&env, &2, &migration_hash);
    /// ```
    pub fn migrate(env: Env, target_version: u32, migration_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Get current version
        let current_version = env.storage().instance().get(&DataKey::Version).unwrap_or(1);

        // Validate target version
        if target_version <= current_version {
            let error_msg = String::from_str(
                &env,
                "Target version must be greater than current version"
            );
            emit_migration_event(
                &env,
                MigrationEvent {
                    from_version: current_version,
                    to_version: target_version,
                    timestamp: env.ledger().timestamp(),
                    migration_hash,
                    success: false,
                    error_message: Some(error_msg),
                },
            );
            panic!("Target version must be greater than current version");
        }

        // Check if migration already completed
        if env.storage().instance().has(&DataKey::MigrationState) {
            let migration_state: MigrationState = env
                .storage()
                .instance()
                .get(&DataKey::MigrationState)
                .unwrap();
            
            if migration_state.to_version >= target_version {
                // Migration already completed, skip
                return;
            }
        }

        // Execute version-specific migrations
        let mut from_version = current_version;
        while from_version < target_version {
            let next_version = from_version + 1;
            
            // Execute migration from from_version to next_version
            match next_version {
                2 => migrate_v1_to_v2(&env),
                3 => migrate_v2_to_v3(&env),
                _ => {
                    let error_msg = String::from_str(
                        &env,
                        "No migration path available"
                    );
                    emit_migration_event(
                        &env,
                        MigrationEvent {
                            from_version,
                            to_version: next_version,
                            timestamp: env.ledger().timestamp(),
                            migration_hash: migration_hash.clone(),
                            success: false,
                            error_message: Some(error_msg),
                        },
                    );
                    panic!("No migration path available");
                }
            }
            
            from_version = next_version;
        }

        // Update version
        env.storage().instance().set(&DataKey::Version, &target_version);

        // Record migration state
        let migration_state = MigrationState {
            from_version: current_version,
            to_version: target_version,
            migrated_at: env.ledger().timestamp(),
            migration_hash: migration_hash.clone(),
        };
        env.storage().instance().set(&DataKey::MigrationState, &migration_state);

        // Emit success event
        emit_migration_event(
            &env,
            MigrationEvent {
                from_version: current_version,
                to_version: target_version,
                timestamp: env.ledger().timestamp(),
                migration_hash: migration_hash.clone(),
                success: true,
                error_message: None,
            },
        );

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("migrate"), admin, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("migrate"), duration);
    }

    /// Gets the current migration state.
    ///
    /// # Returns
    /// * `Option<MigrationState>` - Current migration state if exists
    pub fn get_migration_state(env: Env) -> Option<MigrationState> {
        if env.storage().instance().has(&DataKey::MigrationState) {
            Some(env.storage().instance().get(&DataKey::MigrationState).unwrap())
        } else {
            None
        }
    }

    /// Gets the previous version (before last upgrade).
    ///
    /// # Returns
    /// * `Option<u32>` - Previous version if exists
    pub fn get_previous_version(env: Env) -> Option<u32> {
        if env.storage().instance().has(&DataKey::PreviousVersion) {
            Some(env.storage().instance().get(&DataKey::PreviousVersion).unwrap())
        } else {
            None
        }
    }
}

// ============================================================================
// Migration Functions
// ============================================================================

/// Emits a migration event for audit trail
fn emit_migration_event(env: &Env, event: MigrationEvent) {
    env.events().publish(
        (symbol_short!("migration"),),
        event,
    );
}

/// Migration from version 1 to version 2
/// This is a placeholder migration - add actual data transformation logic here
fn migrate_v1_to_v2(_env: &Env) {
    // Example: Transform old data structures to new ones
    // This is where you would:
    // 1. Read old data format
    // 2. Transform to new format
    // 3. Write new data format
    // 4. Clean up old data if needed
    
    // For now, this is a no-op migration
    // Add actual migration logic based on your data structure changes
}

/// Migration from version 2 to version 3
/// Placeholder for future migrations
fn migrate_v2_to_v3(_env: &Env) {
    // Future migration logic here
    // This will be implemented when v3 is released
}


// ============================================================================
// Testing Module
// ============================================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn multisig_init_works() {
        let env = Env::default();
        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let mut signers = soroban_sdk::Vec::new(&env);
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));
        signers.push_back(Address::generate(&env));

        client.init(&signers, &2u32);
    }

    #[test]
    fn test_set_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        client.set_version(&2);
        assert_eq!(client.get_version(), 2);
    }

    #[test]
    fn test_core_config_snapshot_create_and_restore() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);
        client.set_version(&5);

        let snapshot_id = client.create_config_snapshot();

        client.set_version(&11);
        assert_eq!(client.get_version(), 11);

        client.restore_config_snapshot(&snapshot_id);
        assert_eq!(client.get_version(), 5);
    }

    #[test]
    fn test_core_config_snapshot_prunes_oldest_entries() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        for version in 1..=25u32 {
            client.set_version(&version);
            client.create_config_snapshot();
        }

        let snapshots = client.list_config_snapshots();
        assert_eq!(snapshots.len(), 20);

        let oldest_retained = snapshots.get(0).unwrap();
        assert_eq!(oldest_retained.id, 6);
    }

    #[test]
    fn test_migration_v1_to_v2() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Initial version should be 1
        assert_eq!(client.get_version(), 1);

        // Create migration hash
        let migration_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Migrate to version 2
        client.migrate(&2, &migration_hash);

        // Verify version updated
        assert_eq!(client.get_version(), 2);

        // Verify migration state recorded
        let migration_state = client.get_migration_state();
        assert!(migration_state.is_some());
        let state = migration_state.unwrap();
        assert_eq!(state.from_version, 1);
        assert_eq!(state.to_version, 2);
    }

    #[test]
    #[should_panic(expected = "Target version must be greater than current version")]
    fn test_migration_invalid_target_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let migration_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Try to migrate to version 1 when already at version 1
        client.migrate(&1, &migration_hash);
    }

    #[test]
    fn test_migration_idempotency() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let migration_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Migrate to version 2
        client.migrate(&2, &migration_hash);
        assert_eq!(client.get_version(), 2);

        // Try to migrate again - should be idempotent
        client.migrate(&2, &migration_hash);
        assert_eq!(client.get_version(), 2);

        // Verify migration state unchanged
        let migration_state = client.get_migration_state();
        assert!(migration_state.is_some());
        let state = migration_state.unwrap();
        assert_eq!(state.to_version, 2);
    }

    #[test]
    fn test_get_previous_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Initially no previous version
        assert!(client.get_previous_version().is_none());

        // Simulate upgrade (this would normally be done via upgrade() but we'll set version directly)
        client.set_version(&2);

        // Previous version should still be None unless upgrade() was called
        // This test verifies the get_previous_version function works
    }

    // ========================================================================
    // Integration Tests: Upgrade and Migration Workflow
    // ========================================================================

    #[test]
    fn test_complete_upgrade_and_migration_workflow() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        
        // 1. Initialize contract
        client.init_admin(&admin);
        assert_eq!(client.get_version(), 1);

        // 2. Simulate upgrade (in real scenario, this would call upgrade() with WASM hash)
        // For testing, we'll just test the migration part
        let migration_hash = BytesN::from_array(&env, &[1u8; 32]);

        // 3. Migrate to version 2
        client.migrate(&2, &migration_hash);

        // 4. Verify version updated
        assert_eq!(client.get_version(), 2);

        // 5. Verify migration state recorded
        let migration_state = client.get_migration_state();
        assert!(migration_state.is_some());
        let state = migration_state.unwrap();
        assert_eq!(state.from_version, 1);
        assert_eq!(state.to_version, 2);
        assert!(state.migrated_at > 0);

        // 6. Verify events emitted
        let events = env.events().all();
        assert!(events.len() > 0);
    }

    #[test]
    fn test_migration_sequential_versions() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Migrate from v1 to v2
        let hash1 = BytesN::from_array(&env, &[1u8; 32]);
        client.migrate(&2, &hash1);
        assert_eq!(client.get_version(), 2);

        // Migrate from v2 to v3 (if migration path exists)
        // This would test sequential migrations
        // For now, this will panic as v2->v3 migration is not fully implemented
        // but the structure is there
    }

    #[test]
    fn test_migration_event_emission() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let initial_event_count = env.events().all().len();

        let migration_hash = BytesN::from_array(&env, &[2u8; 32]);
        client.migrate(&2, &migration_hash);

        // Verify migration event was emitted
        let events = env.events().all();
        assert!(events.len() > initial_event_count);
    }

    #[test]
    fn test_admin_initialization() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        assert_eq!(client.get_version(), 2);
    }

    #[test]
    fn test_network_initialization() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let chain_id = String::from_str(&env, "stellar");
        let network_id = String::from_str(&env, "testnet");

        client.init_with_network(&admin, &chain_id, &network_id);

        // Verify initialization
        assert_eq!(client.get_version(), 2);

        // Verify network configuration
        let retrieved_chain = client.get_chain_id();
        let retrieved_network = client.get_network_id();

        assert_eq!(retrieved_chain, Some(chain_id));
        assert_eq!(retrieved_network, Some(network_id));
    }

    #[test]
    fn test_network_info_getter() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let chain_id = String::from_str(&env, "ethereum");
        let network_id = String::from_str(&env, "mainnet");

        client.init_with_network(&admin, &chain_id, &network_id);

        // Test tuple getter
        let (chain, network) = client.get_network_info();
        assert_eq!(chain, Some(chain_id));
        assert_eq!(network, Some(network_id));
    }

    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_cannot_reinitialize_network_config() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let chain_id = String::from_str(&env, "stellar");
        let network_id = String::from_str(&env, "testnet");

        // First initialization should succeed
        client.init_with_network(&admin1, &chain_id, &network_id);

        // Second initialization should panic
        client.init_with_network(&admin2, &chain_id, &network_id);
    }

    #[test]
    fn test_legacy_init_still_works() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);

        // Legacy init should still work (without network config)
        client.init_admin(&admin);

        // Network info should be None for legacy initialization
        assert_eq!(client.get_chain_id(), None);
        assert_eq!(client.get_network_id(), None);
        let (chain, network) = client.get_network_info();
        assert_eq!(chain, None);
        assert_eq!(network, None);
    }

    #[test]
    #[should_panic(expected = "Already initialized")]
    fn test_cannot_reinitialize_admin() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.init_admin(&admin1);
        client.init_admin(&admin2);
    }

    #[test]
    fn test_admin_persists_across_version_updates() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        client.set_version(&3);
        assert_eq!(client.get_version(), 3);

        client.set_version(&4);
        assert_eq!(client.get_version(), 4);
    }

    // ========================================================================
    // Migration Hook Tests (Issue #45)
    // ========================================================================

    #[test]
    fn test_migration_only_runs_once_per_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Verify initial version
        assert_eq!(client.get_version(), 2);

        // Migrate to v3
        let hash = BytesN::from_array(&env, &[1u8; 32]);
        client.migrate(&3, &hash);

        let state1 = client.get_migration_state().unwrap();
        let timestamp1 = state1.migrated_at;

        // Second call with same version - should be idempotent (not re-execute)
        client.migrate(&3, &hash);
        let state2 = client.get_migration_state().unwrap();

        // Verify state unchanged (migration not re-executed)
        assert_eq!(state2.migrated_at, timestamp1);
        assert_eq!(state2.to_version, 3);
    }

    #[test]
    fn test_migration_transforms_state_correctly() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let initial_version = client.get_version();
        assert_eq!(initial_version, 2);

        let hash = BytesN::from_array(&env, &[2u8; 32]);

        // Execute migration to v3
        client.migrate(&3, &hash);

        // Verify transformations
        assert_eq!(client.get_version(), 3);

        let state = client.get_migration_state().unwrap();
        assert_eq!(state.from_version, initial_version);
        assert_eq!(state.to_version, 3);
        assert_eq!(state.migration_hash, hash);
        // Timestamp is set (may be 0 in test environment)
    }

    #[test]
    fn test_migration_requires_admin_authorization() {
        let env = Env::default();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        env.mock_all_auths_allowing_non_root_auth();

        let hash = BytesN::from_array(&env, &[3u8; 32]);

        // This should require admin auth
        client.migrate(&3, &hash);

        // Verify auth was required
        assert!(!env.auths().is_empty());
    }

    #[test]
    #[should_panic(expected = "Target version must be greater than current version")]
    fn test_migration_rejects_downgrade() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        client.set_version(&4);

        let hash = BytesN::from_array(&env, &[4u8; 32]);

        // Try to migrate to lower version - should panic
        client.migrate(&3, &hash);
    }

    #[test]
    fn test_migration_state_persists() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let hash = BytesN::from_array(&env, &[5u8; 32]);
        client.migrate(&3, &hash);

        // Retrieve state multiple times
        let state1 = client.get_migration_state().unwrap();
        let state2 = client.get_migration_state().unwrap();

        assert_eq!(state1.from_version, state2.from_version);
        assert_eq!(state1.to_version, state2.to_version);
        assert_eq!(state1.migrated_at, state2.migrated_at);
        assert_eq!(state1.migration_hash, state2.migration_hash);
    }

    #[test]
    fn test_migration_emits_success_event() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let initial_events = env.events().all().len();

        let hash = BytesN::from_array(&env, &[6u8; 32]);
        client.migrate(&3, &hash);

        let events = env.events().all();
        assert!(events.len() > initial_events);
    }

    #[test]
    fn test_migration_tracks_previous_version() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let v_before = client.get_version();
        assert_eq!(v_before, 2);

        let hash = BytesN::from_array(&env, &[7u8; 32]);
        client.migrate(&3, &hash);

        let state = client.get_migration_state().unwrap();
        assert_eq!(state.from_version, v_before);
        assert_eq!(state.to_version, 3);
    }
    // Export WASM for testing upgrade/rollback scenarios
    // #[cfg(test)]
    // pub const WASM: &[u8] = include_bytes!("../target/wasm32v1-none/release/grainlify_core.wasm");

    #[cfg(test)]
    mod upgrade_rollback_tests;
}

// #[cfg(test)]
// mod migration_hook_tests;
}

