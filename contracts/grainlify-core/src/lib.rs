//! # Grainlify Contract Upgrade System
//!
//! Secure contract upgrade pattern with admin-controlled WASM updates and version tracking.
//! - ❌ Not having a rollback plan

#![no_std]

mod multisig;
use multisig::{ConfigPayload, MultiSig};
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
    String, Symbol, Vec,
};
pub mod asset;
mod governance;
pub mod nonce;
pub mod pseudo_randomness;

pub use governance::{
    Error as GovError, GovernanceConfig, Proposal, ProposalStatus, Vote, VoteType, VotingScheme,
};

// ============================================================================
// Contract Errors
// ============================================================================

/// Typed errors for the GrainlifyContract.
///
/// Using `#[contracterror]` ensures these are surfaced as structured error
/// codes in the Soroban host rather than opaque panic strings, making them
/// easier to handle in client SDKs and off-chain tooling.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// Contract has already been initialized; `init*` cannot be called again.
    AlreadyInitialized = 1,
    /// Caller is not the configured admin.
    NotAdmin = 2,
    /// Contract has not been initialized yet (admin not set).
    NotInitialized = 3,
    /// Multisig threshold has not been reached for this proposal.
    ThresholdNotMet = 4,
    /// The referenced upgrade proposal does not exist.
    ProposalNotFound = 5,
}

// ============================================================================
// Upgrade Event
// ============================================================================

/// Emitted on every successful WASM upgrade (both single-admin and multisig paths).
///
/// Off-chain indexers and monitoring tools should subscribe to the
/// `("upgrade", "wasm")` topic pair to track all contract upgrades.
#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeEvent {
    /// The new WASM hash that was installed.
    pub new_wasm_hash: BytesN<32>,
    /// Version number recorded at the time of upgrade (may be 0 if not yet set).
    pub version: u32,
    /// Ledger timestamp when the upgrade was executed.
    pub timestamp: u64,
}

// ==================== MONITORING MODULE ====================
mod monitoring {
    use super::DataKey;
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Vec};

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

    /// Aggregated performance statistics for a tracked contract function.
    ///
    /// Counters are maintained in persistent storage by [`emit_performance`] and
    /// read back by [`get_performance_stats`].  The storage footprint is bounded
    /// by [`MAX_TRACKED_FUNCTIONS`] — see the eviction policy below.
    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceStats {
        pub function_name: Symbol,
        pub call_count: u64,
        pub total_time: u64,
        pub avg_time: u64,
        pub last_called: u64,
    }

    // Data: Invariant report for external auditors/monitors
    #[contracttype]
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct InvariantReport {
        pub healthy: bool,
        pub config_sane: bool,
        pub metrics_sane: bool,
        pub admin_set: bool,
        pub version_set: bool,
        pub version: u32,
        pub operation_count: u64,
        pub unique_users: u64,
        pub error_count: u64,
        pub violation_count: u32,
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

    /// Maximum number of distinct function names whose performance counters
    /// are retained in persistent storage.  When a new (previously unseen)
    /// function is tracked and the index already contains this many entries,
    /// the **oldest** entry (first element of the `perf_index` vector) is
    /// evicted — its three storage keys (`perf_cnt`, `perf_time`, `perf_last`)
    /// are removed before the new entry is appended.
    ///
    /// This caps total storage at `MAX_TRACKED_FUNCTIONS * 3 + 1` persistent
    /// entries (counters + the index itself), preventing unbounded growth.
    pub const MAX_TRACKED_FUNCTIONS: u32 = 50;

    /// Records a single invocation of `function` with the given `duration`.
    ///
    /// Increments `perf_cnt`, accumulates `perf_time`, and writes
    /// `perf_last` (ledger timestamp) so that [`get_performance_stats`] can
    /// reconstruct the full [`PerformanceStats`] for any tracked function.
    ///
    /// **Note on timestamp granularity:** Within a single ledger close,
    /// `env.ledger().timestamp()` does not advance, so `duration` may be
    /// zero when both start and end are sampled in the same ledger.
    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        // --- eviction bookkeeping -------------------------------------------
        let index_key = Symbol::new(env, "perf_index");
        let mut index: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&index_key)
            .unwrap_or(Vec::new(env));

        // Check whether `function` is already tracked.
        let mut already_tracked = false;
        for i in 0..index.len() {
            if index.get(i).unwrap() == function {
                already_tracked = true;
                break;
            }
        }

        if !already_tracked {
            // Evict the oldest entry when the cap is reached.
            if index.len() >= MAX_TRACKED_FUNCTIONS {
                let oldest = index.get(0).unwrap();
                env.storage()
                    .persistent()
                    .remove(&(Symbol::new(env, "perf_cnt"), oldest.clone()));
                env.storage()
                    .persistent()
                    .remove(&(Symbol::new(env, "perf_time"), oldest.clone()));
                env.storage()
                    .persistent()
                    .remove(&(Symbol::new(env, "perf_last"), oldest.clone()));

                let mut trimmed = Vec::new(env);
                for i in 1..index.len() {
                    trimmed.push_back(index.get(i).unwrap());
                }
                index = trimmed;
            }
            index.push_back(function.clone());
            env.storage().persistent().set(&index_key, &index);
        }

        // --- update counters ------------------------------------------------
        let count_key = (Symbol::new(env, "perf_cnt"), function.clone());
        let time_key = (Symbol::new(env, "perf_time"), function.clone());
        let last_key = (Symbol::new(env, "perf_last"), function.clone());

        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);

        env.storage().persistent().set(&count_key, &(count + 1));
        env.storage()
            .persistent()
            .set(&time_key, &(total + duration));
        env.storage()
            .persistent()
            .set(&last_key, &env.ledger().timestamp());

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

    /// Returns aggregated [`PerformanceStats`] for `function_name`.
    ///
    /// All counters default to `0` when the function has never been tracked,
    /// so this call is always safe (no panics, no zero-division).
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

    /// Verify core monitoring/config invariants.
    /// This is view-only and safe for frequent calls by off-chain monitors.
    pub fn check_invariants(env: &Env) -> InvariantReport {
        let op_key = Symbol::new(env, OPERATION_COUNT);
        let usr_key = Symbol::new(env, USER_COUNT);
        let err_key = Symbol::new(env, ERROR_COUNT);

        let operation_count: u64 = env.storage().persistent().get(&op_key).unwrap_or(0);
        let unique_users: u64 = env.storage().persistent().get(&usr_key).unwrap_or(0);
        let error_count: u64 = env.storage().persistent().get(&err_key).unwrap_or(0);

        let metrics_sane = error_count <= operation_count
            && unique_users <= operation_count
            && (operation_count > 0 || (unique_users == 0 && error_count == 0));

        let admin_set = env.storage().instance().has(&DataKey::Admin);
        let version_opt: Option<u32> = env.storage().instance().get(&DataKey::Version);
        let version_set = version_opt.is_some();
        let version = version_opt.unwrap_or(0);
        let version_sane = version > 0;

        let previous_version_opt: Option<u32> =
            env.storage().instance().get(&DataKey::PreviousVersion);
        let previous_version_sane = match (previous_version_opt, version_opt) {
            (Some(prev), Some(curr)) => prev <= curr,
            (Some(_), None) => false,
            (None, _) => true,
        };

        let chain_id: Option<String> = env.storage().instance().get(&DataKey::ChainId);
        let network_id: Option<String> = env.storage().instance().get(&DataKey::NetworkId);
        let network_pair_sane = match (chain_id, network_id) {
            (Some(chain), Some(network)) => chain.len() > 0 && network.len() > 0,
            (None, None) => true,
            _ => false,
        };

        let config_sane =
            admin_set && version_set && version_sane && previous_version_sane && network_pair_sane;

        let mut violation_count: u32 = 0;
        if !admin_set {
            violation_count += 1;
        }
        if !version_set || !version_sane {
            violation_count += 1;
        }
        if !previous_version_sane {
            violation_count += 1;
        }
        if !network_pair_sane {
            violation_count += 1;
        }
        if error_count > operation_count {
            violation_count += 1;
        }
        if unique_users > operation_count {
            violation_count += 1;
        }
        if operation_count == 0 && (unique_users > 0 || error_count > 0) {
            violation_count += 1;
        }

        InvariantReport {
            healthy: config_sane && metrics_sane,
            config_sane,
            metrics_sane,
            admin_set,
            version_set,
            version,
            operation_count,
            unique_users,
            error_count,
            violation_count,
        }
    }

    pub fn verify_invariants(env: &Env) -> bool {
        check_invariants(env).healthy
    }
}

#[cfg(test)]
mod test_core_monitoring;
#[cfg(test)]
mod test_multisig_config;
#[cfg(test)]
mod test_performance_stats;
#[cfg(test)]
mod test_serialization_compatibility;
#[cfg(test)]
mod test_version_helpers;

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
/// * `MigrationState` - Migration state tracking to prevent double migration
/// * `PreviousVersion` - Tracks previous version for rollback support
/// * `ChainId` - Stores the chain identifier for cross-network protection
/// * `NetworkId` - Stores the network identifier for environment-specific behavior
///
/// # Storage Type
/// Instance storage - Persists across contract upgrades. This is critical for maintaining
/// state continuity when upgrading contract WASM.
///
/// # Storage Key Stability
///
/// **IMPORTANT**: Storage keys must NEVER change between contract versions, as changing
/// keys will cause loss of access to existing data during upgrades. All keys are stable:
///
/// - `Admin` (0): Immutable identifier, safe for all future versions
/// - `Version` (1): Immutable identifier, safe for all future versions
/// - `MigrationState` (3): Immutable identifier, safe for all future versions
/// - `PreviousVersion` (4): May be extended but never renamed
/// - Keys added in future versions should use sequential enum indices
///
/// Any breaking changes to data structures require a migration function in the new WASM.
///
/// # Security Notes
/// - Instance storage persists across WASM upgrades automatically
/// - Admin address (Admin key) is immutable after initialization
/// - Migration state prevents replayed or duplicated migrations
/// - All storage operations are admin-only or derived from admin authorization
#[contracttype]
#[derive(Clone)]
enum DataKey {
    /// Administrator address with upgrade authority
    /// - Immutable after initialization via init_admin()
    /// - Required for all admin operations (upgrade, migrate, set_version)
    /// - Persists across all WASM upgrades
    Admin,

    /// Current version number (increments with upgrades)
    /// - Updated by migrate() and set_version()
    /// - Used to determine which migration functions to execute
    /// - Persists across all WASM upgrades
    Version,

    /// WASM hash stored per proposal (for multisig upgrades)
    UpgradeProposal(u64),

    /// Configuration change payload stored per proposal
    ConfigProposal(u64),

    /// Migration state tracking - prevents double migration
    /// - Set after successful migrate() call
    /// - Records from_version, to_version, timestamp, and migration_hash
    /// - Checked for idempotency in migrate() function
    /// - Persists across all WASM upgrades
    MigrationState,

    /// Previous version before migration (for rollback support)
    /// - Updated by upgrade() function
    /// - Allows comparison before and after WASM upgrade
    /// - Useful for debugging rollback scenarios
    PreviousVersion,

    /// Configuration snapshot data by snapshot id
    /// - Stores point-in-time snapshots of admin/version/multisig config
    /// - Used for recovery and audit trails
    /// - Persists across upgrades
    ConfigSnapshot(u64),

    /// Ordered list of retained snapshot ids
    /// - Maintains order for historical queries
    /// - Limited to CONFIG_SNAPSHOT_LIMIT entries
    /// - Automatically rotates to prevent unbounded storage growth
    SnapshotIndex,

    /// Monotonic snapshot id counter
    /// - Increments with each create_config_snapshot() call
    /// - Ensures snapshot IDs are unique and ordered
    /// - Never decrements, safe for all future versions
    SnapshotCounter,

    /// Chain identifier for cross-network protection
    /// - Set during initialization
    /// - Prevents contract state replay across networks
    /// - Must match network context during execution
    ChainId,

    /// Network identifier for environment-specific behavior
    /// - Distinguishes mainnet from testnet contracts
    /// - May be used for feature flags or behavior divergence
    /// - Persists across upgrades
    NetworkId,

    /// Global fee rate in basis points
    GlobalFeeRate,
    /// Global treasury address
    GlobalTreasuryAddress,
    /// Global pause state
    GlobalPaused,
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
    pub multisig_threshold: u32,
    pub multisig_signers: Vec<Address>,
}

fn contract_is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Version)
        || env.storage().instance().has(&DataKey::Admin)
        || env.storage().instance().has(&DataKey::ChainId)
        || env.storage().instance().has(&DataKey::NetworkId)
        || env.storage().instance().has(&governance::GOVERNANCE_CONFIG)
        || env.storage().instance().has(&governance::PROPOSAL_COUNT)
        || MultiSig::get_config_opt(env).is_some()
}

// ============================================================================
// Migration System
// ============================================================================

/// Migration state tracking to prevent double migration and maintain audit trail.
///
/// # Fields
/// - `from_version`: Version before migration (starting point)
/// - `to_version`: Version after migration (target point)
/// - `migrated_at`: Ledger timestamp when migration completed
/// - `migration_hash`: SHA256 hash of migration data for verification
///
/// # Storage
/// Stored in `DataKey::MigrationState` as instance storage, persists across
/// all WASM upgrades. This is critical for preventing replayed migrations.
///
/// # Idempotency
/// When migrate() is called again with the same target_version, this state
/// is checked first. If to_version == target_version, the call returns early
/// (no-op) to ensure migrations execute exactly once per version boundary.
///
/// # Usage
/// Access via get_migration_state() contract function to verify:
/// - Migration completed successfully
/// - Exact version boundaries involved
/// - Timestamp for audit trail
/// - Hash for verification against external records
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationState {
    /// Version that was migrated from
    pub from_version: u32,
    /// Version that was migrated to
    pub to_version: u32,
    /// Timestamp when migration completed (from env.ledger().timestamp())
    pub migrated_at: u64,
    /// Migration hash for verification and audit trail
    pub migration_hash: BytesN<32>,
}

/// Migration event data emitted for off-chain indexing and audit trail.
///
/// # Events
/// Emitted for every migrate() call, whether successful or failed:
/// - Topic: symbol_short!("migration")
/// - Data: MigrationEvent struct
///
/// # Fields
/// - `from_version`: Starting version before migration
/// - `to_version`: Target version for migration
/// - `timestamp`: Ledger timestamp of migration
/// - `migration_hash`: Verification hash from the call
/// - `success`: Whether migration completed successfully
/// - `error_message`: If success=false, contains error reason
///
/// # Audit Trail
/// All migrations are recorded as events for off-chain monitoring.
/// Failed migrations are also recorded for debugging and security purposes.
#[contracttype]
#[derive(Clone, Debug)]
pub struct MigrationEvent {
    /// Version before migration started
    pub from_version: u32,
    /// Target version for migration
    pub to_version: u32,
    /// Ledger timestamp of migration attempt/completion
    pub timestamp: u64,
    /// Migration data hash for verification
    pub migration_hash: BytesN<32>,
    /// True if migration succeeded, false if failed
    pub success: bool,
    /// Error message if migration failed (None if success=true)
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
    ///
    /// # Panics
    /// * If contract is already initialized (prevents re-init attacks)
    ///
    /// # Security Model
    /// - One-time initialization only
    /// - Sets up multisig configuration for upgrade governance
    /// - All signers must have matching threshold count
    /// - Re-initialization is prevented across all init entrypoints
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect immutability of multisig config
        if contract_is_initialized(&env) {
            panic!("Already initialized");
        }

        // Get representative address for tracking
        let caller = signers.get(0).expect("Signers cannot be empty");

        // Initialize multisig configuration
        MultiSig::init(&env, signers, threshold);

        // Set initial version to mark contract as initialized
        env.storage().instance().set(&DataKey::Version, &VERSION);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init"), caller, true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init"), duration);
    }

    /// Initializes the contract with a single admin address.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform upgrades
    ///
    /// # Panics
    /// * If contract is already initialized (prevents re-init attacks)
    ///
    /// # Security Model
    /// - One-time initialization only
    /// - Admin address is immutable after initialization
    /// - Only single admin, no multisig
    /// - Re-initialization is prevented across all init entrypoints
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage (immutable)
    /// - Sets initial Version number
    /// - Tracks operation metrics
    pub fn init_admin(env: Env, admin: Address) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect admin immutability
        // Check both Admin and Version to be extra safe
        if contract_is_initialized(&env) {
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

    /// Initializes the contract with governance-augmented setup.
    ///
    /// Enables one-time initialization for governance-controlled operations with
    /// optional multisig support. This path is suitable for:
    /// - DAOs requiring voting on upgrades
    /// - Multi-level governance structures
    /// - Hybrid admin + governance models
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address with initial administrative authority
    /// * `config` - Governance configuration (voting period, thresholds, etc.)
    ///
    /// # Panics
    /// * If contract is already initialized (prevents re-init attacks)
    /// * If governance config is invalid (e.g., invalid thresholds)
    ///
    /// # Security Model
    /// - One-time initialization only
    /// - Admin address is immutable after initialization
    /// - Does not require auth during first-call initialization
    /// - Governance parameters are locked after initialization
    /// - Re-initialization is prevented across all init entrypoints
    ///
    /// # Governance Configuration Validation
    /// - `quorum_percentage`: Must be 0-10000 (basis points, 0-100%)
    /// - `approval_threshold`: Must be 0-10000 (basis points, 0-100%)
    /// - `voting_period`: May be zero if immediate-close voting is desired
    /// - `voting_scheme`: OnePersonOneVote or TokenWeighted
    ///
    /// # State Changes
    /// - Sets Admin address in instance storage (immutable)
    /// - Initializes governance configuration
    /// - Sets initial Version number
    /// - Initializes proposal counter to 0
    /// - Tracks operation metrics
    ///
    /// # Initialization Matrix
    /// | Path | Admin | Multisig | Governance | Use Case |
    /// |------|-------|----------|------------|----------|
    /// | init | ❌ | ✅ | ❌ | M-of-N multisig upgrades |
    /// | init_admin | ✅ | ❌ | ❌ | Single-admin trusted setup |
    /// | init_governance | ✅ | ❌ | ✅ | DAO governance upgrades |
    ///
    /// # Example
    /// ```rust
    /// use soroban_sdk::{Address, Env};
    /// use grainlify_core::{GovernanceConfig, VotingScheme};
    ///
    /// let env = Env::default();
    /// let admin = Address::generate(&env);
    ///
    /// let gov_config = GovernanceConfig {
    ///     voting_period: 86400,        // 24 hours
    ///     execution_delay: 3600,       // 1 hour after approval
    ///     quorum_percentage: 4000,     // 40% quorum
    ///     approval_threshold: 6000,    // 60% approval needed
    ///     min_proposal_stake: 1000,    // 1000 units stake
    ///     voting_scheme: VotingScheme::OnePersonOneVote,
    /// };
    ///
    /// // Initialize with governance
    /// contract.init_governance(&env, &admin, &gov_config);
    ///
    /// // Subsequent init attempts will panic
    /// // contract.init_governance(...); // ❌ Panics!
    /// ```
    ///
    /// # Gas Cost
    /// Medium - Multiple storage writes for governance config and version
    ///
    /// # Authorization
    /// - No authorization required during initialization (first-caller pattern)
    /// - Admin authorization required for governance operations after init
    ///
    /// # Events
    /// Emits initialization tracking metrics for monitoring
    pub fn init_governance(env: Env, admin: Address, config: GovernanceConfig) {
        let start = env.ledger().timestamp();

        // Prevent re-initialization to protect immutability
        if contract_is_initialized(&env) {
            monitoring::track_operation(&env, symbol_short!("init_gov"), admin.clone(), false);
            panic!("Already initialized");
        }

        match governance::validate_config(&config) {
            Ok(()) => {}
            Err(governance::Error::InvalidThreshold) => {
                monitoring::track_operation(&env, symbol_short!("init_gov"), admin.clone(), false);
                panic!("Invalid governance threshold");
            }
            Err(governance::Error::ThresholdTooLow) => {
                monitoring::track_operation(&env, symbol_short!("init_gov"), admin.clone(), false);
                panic!("Approval threshold too low (minimum 50%)");
            }
            Err(_) => {
                monitoring::track_operation(&env, symbol_short!("init_gov"), admin.clone(), false);
                panic!("Invalid governance configuration");
            }
        }

        // Store admin address (immutable after this point)
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Initialize governance configuration
        env.storage()
            .instance()
            .set(&governance::GOVERNANCE_CONFIG, &config);

        // Initialize proposal counter
        env.storage()
            .instance()
            .set(&governance::PROPOSAL_COUNT, &0u32);

        // Set initial version
        env.storage().instance().set(&DataKey::Version, &VERSION);

        // Track successful operation
        monitoring::track_operation(&env, symbol_short!("init_gov"), admin.clone(), true);

        // Track performance
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("init_gov"), duration);

        // Emit initialization event
        env.events().publish(
            (symbol_short!("init"), symbol_short!("gov")),
            (admin, config.voting_period, config.approval_threshold),
        );
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
    pub fn propose_upgrade(env: Env, proposer: Address, wasm_hash: BytesN<32>) -> u64 {
        let proposal_id = MultiSig::propose(&env, proposer);

        env.storage()
            .instance()
            .set(&DataKey::UpgradeProposal(proposal_id), &wasm_hash);

        proposal_id
    }

    /// Approves an upgrade proposal (multisig version).
    pub fn approve_upgrade(env: Env, proposal_id: u64, signer: Address) {
        MultiSig::approve(&env, proposal_id, signer);
    }

    /// Proposes a configuration change (multisig version).
    pub fn propose_config_change(env: Env, proposer: Address, payload: ConfigPayload) -> u64 {
        let proposal_id = MultiSig::propose(&env, proposer);
        env.storage()
            .instance()
            .set(&DataKey::ConfigProposal(proposal_id), &payload);
        proposal_id
    }

    /// Approves a configuration change proposal (multisig version).
    pub fn approve_config_change(env: Env, proposal_id: u64, signer: Address) {
        MultiSig::approve(&env, proposal_id, signer);

        if MultiSig::can_execute(&env, proposal_id) {
            let payload: ConfigPayload = env
                .storage()
                .instance()
                .get(&DataKey::ConfigProposal(proposal_id))
                .unwrap();
            Self::execute_config_change(&env, payload);
            MultiSig::mark_executed(&env, proposal_id);

            // Emit execution event
            env.events().publish(
                (symbol_short!("cfg_exec"),),
                (proposal_id, env.ledger().timestamp()),
            );
        }
    }

    /// Cancels a configuration change proposal (multisig version).
    pub fn cancel_config_change(env: Env, proposal_id: u64, canceller: Address) {
        MultiSig::cancel(&env, proposal_id, canceller);
    }

    fn execute_config_change(env: &Env, payload: ConfigPayload) {
        match payload {
            ConfigPayload::LockFeeRate(rate) => {
                env.storage().instance().set(&DataKey::GlobalFeeRate, &rate);
            }
            ConfigPayload::PayoutFeeRate(_rate) => {
                // Not implemented globally, but could be
            }
            ConfigPayload::FeeRecipient(recipient) => {
                env.storage()
                    .instance()
                    .set(&DataKey::GlobalTreasuryAddress, &recipient);
            }
            ConfigPayload::FeesEnabled(_enabled) => {
                // Not implemented globally
            }
            ConfigPayload::Admin(admin) => {
                env.storage().instance().set(&DataKey::Admin, &admin);
            }
            ConfigPayload::Pause(lock, _release, _refund) => {
                env.storage().instance().set(&DataKey::GlobalPaused, &lock);
            }
        }
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
    ///
    /// # Authorization
    /// No additional auth required here — the multisig approval quorum
    /// (collected via [`approve_upgrade`]) acts as the authorization gate.
    /// Panics with `"Threshold not met"` if quorum has not been reached.
    ///
    /// # Events
    /// Emits `("upgrade", "wasm")` → [`UpgradeEvent`] on success.
    ///
    /// # State Preservation
    /// All instance storage is preserved across the WASM replacement.
    pub fn execute_upgrade(env: Env, proposal_id: u64) {
        if !MultiSig::can_execute(&env, proposal_id) {
            panic!("Threshold not met");
        }

        let wasm_hash: BytesN<32> = env
            .storage()
            .instance()
            .get(&DataKey::UpgradeProposal(proposal_id))
            .expect("Missing upgrade proposal");

        // Store previous version for potential rollback
        let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(1);
        env.storage()
            .instance()
            .set(&DataKey::PreviousVersion, &current_version);

        // Perform WASM upgrade — instance storage is preserved
        env.deployer()
            .update_current_contract_wasm(wasm_hash.clone());

        // Emit structured upgrade event for off-chain indexers
        env.events().publish(
            (symbol_short!("upgrade"), symbol_short!("wasm")),
            UpgradeEvent {
                new_wasm_hash: wasm_hash,
                version: current_version,
                timestamp: env.ledger().timestamp(),
            },
        );

        MultiSig::mark_executed(&env, proposal_id);
    }

    /// Upgrades the contract to new WASM code (single admin version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    /// Upgrades the contract to new WASM code (single admin version).
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `new_wasm_hash` - Hash of the uploaded WASM code (32 bytes)
    ///
    /// # Authorization
    /// Requires `require_auth()` from the configured admin address.
    /// Panics with [`ContractError::NotInitialized`] if the contract has not
    /// been initialized, or [`ContractError::NotAdmin`] if the caller is not
    /// the admin.
    ///
    /// # Events
    /// Emits `("upgrade", "wasm")` → [`UpgradeEvent`] on success.
    ///
    /// # State Preservation
    /// All instance storage (admin, version, migration state, snapshots) is
    /// preserved across the WASM replacement — only the executable code changes.
    /// Returns the configured admin address, or `None` if the contract has not
    /// been initialized via [`init_admin`] or [`init_with_network`].
    ///
    /// This is a view-only helper for off-chain tooling and auditors.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        // Verify admin is set (contract initialized).
        // Panics with ContractError::NotInitialized if admin key is absent.
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("{}", ContractError::NotInitialized as u32));

        // Require admin authorization — panics with auth error if not signed by admin.
        // This is the primary security gate: only the configured admin can upgrade.
        admin.require_auth();

        // Store previous version for potential rollback
        let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(1);
        env.storage()
            .instance()
            .set(&DataKey::PreviousVersion, &current_version);

        // Perform WASM upgrade — instance storage is preserved
        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        // Emit structured upgrade event for off-chain indexers
        env.events().publish(
            (symbol_short!("upgrade"), symbol_short!("wasm")),
            UpgradeEvent {
                new_wasm_hash,
                version: current_version,
                timestamp: env.ledger().timestamp(),
            },
        );

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

    /// Returns the semantic version string decoded from the stored numeric encoding.
    ///
    /// # Encoding policy
    /// Versions stored as `major * 10_000 + minor * 100 + patch` are decoded
    /// directly.  Legacy single-digit values (`1`, `2`, …) are treated as
    /// `major.0.0` for backward compatibility.  A stored value of `0` returns
    /// `"0.0.0"`.
    ///
    /// # Examples
    /// | stored value | returned string |
    /// |---|---|
    /// | `0`      | `"0.0.0"` |
    /// | `1`      | `"1.0.0"` (legacy) |
    /// | `10000`  | `"1.0.0"` |
    /// | `10100`  | `"1.1.0"` |
    /// | `10001`  | `"1.0.1"` |
    /// | `20305`  | `"2.3.5"` |
    ///
    /// # Security note
    /// This is a pure view function; it performs no authorization and cannot
    /// mutate state.
    pub fn get_version_semver_string(env: Env) -> String {
        let encoded = Self::get_version_numeric_encoded(env.clone());
        let major = encoded / 10_000;
        let minor = (encoded % 10_000) / 100;
        let patch = encoded % 100;

        // Build "major.minor.patch" without heap allocation beyond the SDK String.
        // Maximum length: 3 digits each + 2 dots = 11 chars — well within limits.
        let mut buf = [0u8; 12];
        let mut pos = 0usize;

        let write_u32 = |n: u32, buf: &mut [u8; 12], pos: &mut usize| {
            if n >= 100 {
                buf[*pos] = b'0' + (n / 100) as u8;
                *pos += 1;
            }
            if n >= 10 {
                buf[*pos] = b'0' + ((n % 100) / 10) as u8;
                *pos += 1;
            }
            buf[*pos] = b'0' + (n % 10) as u8;
            *pos += 1;
        };

        write_u32(major, &mut buf, &mut pos);
        buf[pos] = b'.';
        pos += 1;
        write_u32(minor, &mut buf, &mut pos);
        buf[pos] = b'.';
        pos += 1;
        write_u32(patch, &mut buf, &mut pos);

        // SAFETY: buf contains only ASCII digits and dots.
        let s = core::str::from_utf8(&buf[..pos]).unwrap_or("0.0.0");
        String::from_str(&env, s)
    }

    /// Returns the stored version normalised to the `major * 10_000 + minor * 100 + patch`
    /// encoding used throughout this contract.
    ///
    /// Legacy single-digit values (`1`, `2`, …, `9_999`) are promoted to
    /// `value * 10_000` so that comparisons with fully-encoded versions are
    /// always correct.
    ///
    /// # Examples
    /// ```text
    /// stored 1     → 10_000   (1.0.0)
    /// stored 2     → 20_000   (2.0.0)
    /// stored 10100 → 10_100   (1.1.0)  — already encoded, returned as-is
    /// stored 0     → 0        (0.0.0)
    /// ```
    ///
    /// # Security note
    /// Pure view function; no auth required, no state mutation.
    pub fn get_version_numeric_encoded(env: Env) -> u32 {
        let raw: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
        if raw >= 10_000 {
            raw
        } else {
            raw.saturating_mul(10_000)
        }
    }

    /// Asserts that the contract's current version is at least `min_numeric`.
    ///
    /// `min_numeric` must use the `major * 10_000 + minor * 100 + patch`
    /// encoding (e.g., `20000` for ≥ 2.0.0, `10100` for ≥ 1.1.0).
    ///
    /// # Errors
    /// Panics with [`ContractError::NotInitialized`] (code `3`) when the
    /// contract has not been initialised yet (version == 0).
    ///
    /// Panics with the string `"version_too_low"` when the current encoded
    /// version is strictly less than `min_numeric`.  Integrators should catch
    /// this panic string to detect version incompatibility.
    ///
    /// # Examples
    /// ```text
    /// // Contract at 2.0.0 (stored as 20000 or legacy 2)
    /// require_min_version(20000)  → ok
    /// require_min_version(10000)  → ok   (1.0.0 ≤ 2.0.0)
    /// require_min_version(20001)  → panic("version_too_low")
    /// ```
    ///
    /// # Security note
    /// Pure view function; no auth required, no state mutation.
    pub fn require_min_version(env: Env, min_numeric: u32) {
        let cur = Self::get_version_numeric_encoded(env.clone());
        if cur == 0 {
            panic!("{}", ContractError::NotInitialized as u32);
        }
        if cur < min_numeric {
            panic!("version_too_low");
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

    /// Creates an on-chain snapshot of critical core configuration (admin-only).
    /// Returns snapshot id.
    pub fn create_config_snapshot(env: Env) -> u64 {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();

        let next_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SnapshotCounter)
            .unwrap_or(0)
            + 1;

        let (multisig_threshold, multisig_signers) = match MultiSig::get_config_opt(&env) {
            Some(cfg) => (cfg.threshold, cfg.signers),
            None => (0u32, Vec::new(&env)),
        };

        let snapshot = CoreConfigSnapshot {
            id: next_id,
            timestamp: env.ledger().timestamp(),
            admin: env.storage().instance().get(&DataKey::Admin),
            version: env.storage().instance().get(&DataKey::Version).unwrap_or(0),
            previous_version: env.storage().instance().get(&DataKey::PreviousVersion),
            multisig_threshold,
            multisig_signers,
        };

        env.storage()
            .instance()
            .set(&DataKey::ConfigSnapshot(next_id), &snapshot);

        let mut index: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SnapshotIndex)
            .unwrap_or(Vec::new(&env));
        index.push_back(next_id);

        if index.len() > CONFIG_SNAPSHOT_LIMIT {
            let oldest_snapshot_id = index.get(0).unwrap();
            env.storage()
                .instance()
                .remove(&DataKey::ConfigSnapshot(oldest_snapshot_id));

            let mut trimmed = Vec::new(&env);
            for i in 1..index.len() {
                trimmed.push_back(index.get(i).unwrap());
            }
            index = trimmed;
        }

        env.storage()
            .instance()
            .set(&DataKey::SnapshotIndex, &index);
        env.storage()
            .instance()
            .set(&DataKey::SnapshotCounter, &next_id);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("create")),
            (next_id, snapshot.timestamp),
        );

        next_id
    }

    /// Lists retained config snapshots in oldest-to-newest order.
    pub fn list_config_snapshots(env: Env) -> Vec<CoreConfigSnapshot> {
        let index: Vec<u64> = env
            .storage()
            .instance()
            .get(&DataKey::SnapshotIndex)
            .unwrap_or(Vec::new(&env));

        let mut snapshots: Vec<CoreConfigSnapshot> = Vec::new(&env);
        for snapshot_id in index.iter() {
            if let Some(snapshot) = env
                .storage()
                .instance()
                .get::<DataKey, CoreConfigSnapshot>(&DataKey::ConfigSnapshot(snapshot_id))
            {
                snapshots.push_back(snapshot);
            }
        }
        snapshots
    }

    /// Retrieves the chain identifier.
    pub fn get_chain_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::ChainId)
    }

    /// Retrieves the network identifier.
    pub fn get_network_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::NetworkId)
    }

    /// Retrieves both chain and network identifiers as a tuple.
    pub fn get_network_info(env: Env) -> (Option<String>, Option<String>) {
        let chain_id = env.storage().instance().get(&DataKey::ChainId);
        let network_id = env.storage().instance().get(&DataKey::NetworkId);
        (chain_id, network_id)
    }

    /// Initializes the contract with admin and optional chain/network configuration.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `admin` - Address authorized to perform operations
    /// * `chain_id` - Optional chain identifier (e.g., "stellar", "ethereum")
    /// * `network_id` - Optional network identifier (e.g., "mainnet", "testnet")
    pub fn init_with_network(env: Env, admin: Address, chain_id: String, network_id: String) {
        if contract_is_initialized(&env) {
            panic!("Already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
        env.storage().instance().set(&DataKey::ChainId, &chain_id);
        env.storage()
            .instance()
            .set(&DataKey::NetworkId, &network_id);
    }

    /// Restores core configuration from a previously captured snapshot (admin-only).
    pub fn restore_config_snapshot(env: Env, snapshot_id: u64) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");
        admin.require_auth();

        let snapshot: CoreConfigSnapshot = env
            .storage()
            .instance()
            .get(&DataKey::ConfigSnapshot(snapshot_id))
            .unwrap_or_else(|| panic!("Snapshot not found"));

        if let Some(snapshot_admin) = snapshot.admin {
            env.storage()
                .instance()
                .set(&DataKey::Admin, &snapshot_admin);
        } else {
            env.storage().instance().remove(&DataKey::Admin);
        }

        env.storage()
            .instance()
            .set(&DataKey::Version, &snapshot.version);

        match snapshot.previous_version {
            Some(prev) => env
                .storage()
                .instance()
                .set(&DataKey::PreviousVersion, &prev),
            None => env.storage().instance().remove(&DataKey::PreviousVersion),
        }

        if snapshot.multisig_threshold > 0 {
            let config = multisig::MultiSigConfig {
                signers: snapshot.multisig_signers.clone(),
                threshold: snapshot.multisig_threshold,
            };
            MultiSig::set_config(&env, config);
        } else {
            MultiSig::clear_config(&env);
        }

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("restore")),
            (snapshot_id, env.ledger().timestamp()),
        );
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

    /// Returns aggregated performance statistics for `function_name`.
    ///
    /// Counters default to zero when no data has been recorded, so this
    /// endpoint is always safe to call for any symbol.
    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    /// Return a detailed invariant report for auditors and monitoring tools.
    pub fn check_invariants(env: Env) -> monitoring::InvariantReport {
        monitoring::check_invariants(&env)
    }

    /// Lightweight invariant verdict for frequent monitoring calls.
    pub fn verify_invariants(env: Env) -> bool {
        monitoring::verify_invariants(&env)
    }

    // ========================================================================
    // State Migration System
    // ========================================================================

    /// Executes state migration from current version to target version.
    ///
    /// # Arguments
    /// * `env` - The contract environment
    /// * `target_version` - Version to migrate to (must be > current version)
    /// * `migration_hash` - Hash of migration data for verification (32 bytes)
    ///
    /// # Authorization
    /// - REQUIRED: Only admin (set via init_admin) can call this function
    /// - REQUIRED: Admin must sign the transaction (enforce_auth)
    /// - EFFECT: Panics if caller is not the admin
    ///
    /// # State Changes (Idempotent)
    /// - **First call**: Executes version-specific migration functions
    /// - **First call**: Updates `DataKey::Version` to target_version
    /// - **First call**: Stores migration state in `DataKey::MigrationState`
    /// - **First call**: Emits successful migration event
    /// - **Retry with same target**: Returns immediately (no-op, no events)
    /// - **Retry with different target**: Panics (prevents confusion)
    ///
    /// # Storage Keys Involved
    /// - `DataKey::Admin`: Retrieved for authorization check
    /// - `DataKey::Version`: Read for current version, updated with target
    /// - `DataKey::MigrationState`: Checked for idempotency, set after migration
    /// - `DataKey::PreviousVersion`: May be updated by migration functions
    ///
    /// # Migration Chain Logic
    /// If target_version > current_version + 1, executes intermediate migrations:
    /// - v1 → v2 calls migrate_v1_to_v2()
    /// - v2 → v3 calls migrate_v2_to_v3()
    /// - v1 → v3 calls both in sequence
    ///
    /// # Version Control
    /// - Returns error if target_version <= current_version
    /// - Returns error if no migration path exists for version jump
    /// - Ensures monotonically increasing version numbers
    ///
    /// # Audit Trail
    /// - Emits MigrationEvent with from_version, to_version, timestamp, success flag
    /// - Calls monitoring::track_operation for operation tracking
    /// - Calls monitoring::emit_performance for gas accounting
    ///
    /// # Idempotency Guarantee
    ///
    /// ```rust
    /// // Safe to retry: second call is a no-op
    /// client.migrate(&3, &hash1);
    /// client.migrate(&3, &hash1);  // Returns early, no events emitted
    ///
    /// // Different hash for same target: still returns early
    /// client.migrate(&3, &hash2);  // Returns early, preserves original hash
    ///
    /// // Different target: panics with version check error
    /// client.migrate(&3, &hash1);
    /// client.migrate(&2, &hash1);  // Panics: "Target version must be greater"
    /// ```
    ///
    /// # Security Considerations
    /// 1. **Replay Protection**: Migration hash is stored and verified offline
    /// 2. **Admin Control**: Only admin can trigger migrations
    /// 3. **Version Monotonicity**: Cannot downgrade, forward-only migrations
    /// 4. **State Isolation**: Old keys preserved (no key mutations except version)
    /// 5. **Pre-WASM Upgrade**: Call migrate() AFTER uploading new WASM to update state
    ///
    /// # Storage Stability
    /// This function does NOT modify the DataKey enum or rename keys.
    /// Storage keys remain stable across all past and future versions:
    /// - DataKey::Admin (0)
    /// - DataKey::Version (1)
    /// - DataKey::MigrationState (3)
    /// - All other keys retain their enum variants
    ///
    /// # Failure Modes
    /// - Panics if admin is not set (contract not initialized)
    /// - Panics if caller is not admin (admin.require_auth fails)
    /// - Panics if target_version <= current_version
    /// - Panics if no migration path exists (e.g., v3 → v4 with no migrate_v3_to_v4)
    ///
    /// # Performance
    /// - Gas: Proportional to migrations executed (typically 1-3 per call)
    /// - Storage: Constant (same keys updated each call)
    /// - Can be safely called multiple times with same arguments
    pub fn migrate(env: Env, target_version: u32, migration_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        // Verify admin authorization
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        // Get current version
        let current_version = env.storage().instance().get(&DataKey::Version).unwrap_or(1);

        // Validate target version
        if target_version <= current_version {
            let error_msg =
                String::from_str(&env, "Target version must be greater than current version");
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
                    let error_msg = String::from_str(&env, "No migration path available");
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
        env.storage()
            .instance()
            .set(&DataKey::Version, &target_version);

        // Record migration state
        let migration_state = MigrationState {
            from_version: current_version,
            to_version: target_version,
            migrated_at: env.ledger().timestamp(),
            migration_hash: migration_hash.clone(),
        };
        env.storage()
            .instance()
            .set(&DataKey::MigrationState, &migration_state);

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
            Some(
                env.storage()
                    .instance()
                    .get(&DataKey::MigrationState)
                    .unwrap(),
            )
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
            Some(
                env.storage()
                    .instance()
                    .get(&DataKey::PreviousVersion)
                    .unwrap(),
            )
        } else {
            None
        }
    }
}

// ── UpgradeInterface conformance (Issue #574) ───────────────────────────────

pub mod traits {
    use soroban_sdk::{Env, String};

    /// Upgrade interface — mirrors bounty_escrow traits.rs definition.
    /// Kept local to avoid cross-crate dependency.
    pub trait UpgradeInterface {
        fn get_version(env: &Env) -> u32;
        fn set_version(env: &Env, new_version: u32) -> Result<(), String>;
    }
}

#[cfg(feature = "contract")]
impl traits::UpgradeInterface for GrainlifyContract {
    fn get_version(env: &Env) -> u32 {
        GrainlifyContract::get_version(env.clone())
    }

    fn set_version(env: &Env, new_version: u32) -> Result<(), soroban_sdk::String> {
        // set_version panics if admin auth fails; surface as Err for trait callers.
        GrainlifyContract::set_version(env.clone(), new_version);
        Ok(())
    }
}

// ============================================================================
// Migration Functions
// ============================================================================

/// Emits a migration event for audit trail and off-chain indexing.
///
/// # Arguments
/// * `env` - The contract environment
/// * `event` - MigrationEvent struct containing migration details
///
/// # Event Topic
/// Published with topic: symbol_short!("migration")
///
/// # Off-Chain Integration
/// Indexers should listen for events with topic "migration" to maintain
/// an audit trail of all migration attempts (both successful and failed).
fn emit_migration_event(env: &Env, event: MigrationEvent) {
    env.events().publish((symbol_short!("migration"),), event);
}

/// Migration from version 1 to version 2 - Storage transformation handler.
///
/// # Purpose
/// This function is called when migrating from v1 to v2. It handles any
/// data structure transformations or state reorganization needed.
///
/// # Current Implementation
/// This is a placeholder with no-op implementation. When v2 breaking changes
/// are defined, implement the following pattern:
///
/// ```rust
/// // 1. Read old data format from storage
/// let old_data: OldStructure = env.storage().instance().get(&DataKey::OldKey)?;
///
/// // 2. Transform to new format
/// let new_data = transform_to_new_format(old_data);
///
/// // 3. Write new format
/// env.storage().instance().set(&DataKey::NewKey, &new_data);
///
/// // 4. Optionally clean up old keys
/// env.storage().instance().remove(&DataKey::OldKey);
/// ```
///
/// # Important Notes
/// - NEVER rename or remove DataKey enum variants (breaks storage stability)
/// - Use new enum variants for new storage keys in future migrations
/// - Document all data transformations with examples
/// - Test migrations thoroughly on testnet before mainnet deployment
/// - Store migration data on-chain for audit trail verification
///
/// # Security Considerations
/// - Storage keys must remain stable across all versions
/// - Data transformations must be idempotent (can replay without issues)
/// - Invalid data should be explicitly handled (panic or default)
/// - Admin authorization is already verified by the migrate() function
fn migrate_v1_to_v2(_env: &Env) {
    // Placeholder migration - add actual data structure transformations here
    // when v2 includes breaking changes to contract state.
    //
    // Current implementation effect: No-op (v1 storage layout compatible with v2)
    //
    // Future implementations should follow this pattern:
    // - Read old entities
    // - Transform schemas
    // - Write new entities
    // - Clean up old keys (optional, to save storage)
}

/// Migration from version 2 to version 3 - Storage transformation handler.
///
/// # Purpose
/// This function is called when migrating from v2 to v3. It handles any
/// data structure transformations or state reorganization needed.
///
/// # Current Implementation
/// This is a placeholder with no-op implementation. When v3 breaking changes
/// are defined, implement similar to migrate_v1_to_v2().
///
/// # Storage Key Guarantee
/// All existing storage keys from v1 and v2 remain valid:
/// - DataKey::Admin (unchanged, immutable)
/// - DataKey::Version (unchanged, managed by migrate())
/// - DataKey::MigrationState (unchanged, tracks migration history)
/// - All user-defined keys (backward compatible)
///
/// # Adding New Features in v3+
/// If v3 introduces new contract functionality:
/// 1. Define new DataKey variants (new enum indices don't conflict)
/// 2. Initialize new storage in migrate_v2_to_v3() if needed
/// 3. Use get().unwrap_or(default) for missing keys in existing code
/// 4. Document new keys in DataKey enum comments
fn migrate_v2_to_v3(_env: &Env) {
    // Placeholder migration - add actual data structure transformations here
    // when v3 includes breaking changes to contract state.
    //
    // Current implementation effect: No-op (v2 storage layout compatible with v3)
}

// ============================================================================
// Testing Module
// ============================================================================
#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Env,
    };

    // Include end-to-end upgrade and migration tests
    pub mod e2e_upgrade_migration_tests;
    pub mod invariant_entrypoints_tests;
    pub mod upgrade_rollback_tests;

    // WASM for testing (only available after building for wasm32 target)
    #[cfg(feature = "upgrade_rollback_tests")]
    pub const WASM: &[u8] =
        include_bytes!("../target/wasm32-unknown-unknown/release/grainlify_core.wasm");

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
    fn test_migration_v2_to_v3() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        // Initial version should be 2
        assert_eq!(client.get_version(), 2);

        // Create migration hash
        let migration_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Migrate to version 3
        client.migrate(&3, &migration_hash);

        // Verify version updated
        assert_eq!(client.get_version(), 3);

        // Verify migration state recorded
        let migration_state = client.get_migration_state();
        assert!(migration_state.is_some());
        let state = migration_state.unwrap();
        assert_eq!(state.from_version, 2);
        assert_eq!(state.to_version, 3);
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
    #[should_panic(expected = "Target version must be greater than current version")]
    fn test_migration_repeated_same_version_rejected() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register_contract(None, GrainlifyContract);
        let client = GrainlifyContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.init_admin(&admin);

        let migration_hash = BytesN::from_array(&env, &[0u8; 32]);

        // Migrate to version 3
        client.migrate(&3, &migration_hash);
        assert_eq!(client.get_version(), 3);

        // Repeating same target is rejected by current migration guard
        client.migrate(&3, &migration_hash);
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
        assert_eq!(client.get_version(), 2);

        // 2. Simulate upgrade (in real scenario, this would call upgrade() with WASM hash)
        // For testing, we'll just test the migration part
        let migration_hash = BytesN::from_array(&env, &[1u8; 32]);

        // 3. Migrate to version 3
        client.migrate(&3, &migration_hash);

        // 4. Verify version updated
        assert_eq!(client.get_version(), 3);

        // 5. Verify migration state recorded
        let migration_state = client.get_migration_state();
        assert!(migration_state.is_some());
        let state = migration_state.unwrap();
        assert_eq!(state.from_version, 2);
        assert_eq!(state.to_version, 3);

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

        // Migrate from v2 to v3
        let hash1 = BytesN::from_array(&env, &[1u8; 32]);
        client.migrate(&3, &hash1);
        assert_eq!(client.get_version(), 3);
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
        client.migrate(&3, &migration_hash);

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
    #[should_panic(expected = "Target version must be greater than current version")]
    fn test_migration_rejects_repeat_for_same_target_version() {
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

        // Second call with same version is rejected by current migration guard
        client.migrate(&3, &hash);
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
}
