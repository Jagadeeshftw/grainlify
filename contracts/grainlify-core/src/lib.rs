#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol};
pub mod asset;
pub mod commit_reveal;
pub mod error_registry;
pub mod errors;
mod governance;
pub mod nonce;
pub mod pseudo_randomness;
pub mod strict_mode;

#[cfg(test)]
mod test_error_registry;

pub use governance::{GovernanceConfig, Proposal, ProposalStatus, Vote, VoteType, VotingScheme};

// ============================================================================
// Contract Errors
// ============================================================================

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    NotAdmin = 3,
    ThresholdNotMet = 101,
    ProposalNotFound = 102,
    /// [FIX-C01] Migration hash commitment not found — must call commit_migration first
    MigrationCommitmentNotFound = 103,
    /// [FIX-C01] Migration hash does not match committed hash
    MigrationHashMismatch = 104,
    /// [FIX-H02] Timelock delay exceeds maximum allowed value
    TimelockDelayTooHigh = 105,
    /// [FIX-C02] Snapshot restoring admin requires two-step confirmation
    SnapshotRestoreAdminPending = 106,
    /// Snapshot was pruned and is no longer available
    SnapshotPruned = 107,
}
pub const STORAGE_SCHEMA_VERSION: u32 = 1;
pub const LIVENESS_SCHEMA_VERSION: u32 = 1;
const CONFIG_SNAPSHOT_LIMIT: u32 = 20;

/// Maximum number of deployed contracts that can be registered.
/// Prevents unbounded storage growth and ensures predictable gas costs.
const MAX_DEPLOYED_CONTRACTS: u32 = 200;

/// Default timelock delay for upgrade execution (24 hours in seconds)
const DEFAULT_TIMELOCK_DELAY: u64 = 86_400;

/// [FIX-H02] Maximum allowed timelock delay (30 days) — prevents bricking upgrades
const MAX_TIMELOCK_DELAY: u64 = 2_592_000;

/// [FIX-H02] Minimum allowed timelock delay (1 hour)
const MIN_TIMELOCK_DELAY: u64 = 3_600;

// ============================================================================
// Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct UpgradeEvent {
    /// The new WASM hash that was installed.
    pub new_wasm_hash: BytesN<32>,
    /// Version number recorded at the time of upgrade (may be 0 if not yet set).
    pub previous_version: u32,
    /// Ledger timestamp when the upgrade was executed.
    pub timestamp: u64,
}

/// Emitted when read-only mode is toggled.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadOnlyModeEvent {
    pub enabled: bool,
    pub admin: Address,
    pub timestamp: u64,
}

/// Emitted during contract initialization to record build and deployment information.
///
/// This event provides crucial metadata for auditing and monitoring contract deployments:
/// - Allows indexers and monitoring systems to track contract initialization events
/// - Records the initial admin address for access control auditing
/// - Captures the exact ledger timestamp for event sequencing
/// - Enables verification of deployment order and timing across networks
///
/// # Security Considerations
/// - Event is emitted during `init_admin` which requires the admin's authorization
/// - Provides transparent audit trail for deployment activities
/// - Should be indexed by off-chain monitoring systems for initialization verification
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildInfoEvent {
    /// The admin address that authorized contract initialization
    pub admin: Address,
    /// Initial contract version set during initialization
    pub version: u32,
    /// Ledger timestamp when the contract was initialized
    pub timestamp: u64,
}

/// Point-in-time snapshot of core configuration.
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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotDiff {
    pub from_id: u64,
    pub to_id: u64,
    pub admin_changed: bool,
    pub version_changed: bool,
    pub previous_version_changed: bool,
    pub multisig_threshold_changed: bool,
    pub multisig_signers_changed: bool,
    pub from_version: u32,
    pub to_version: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackInfo {
    pub current_version: u32,
    pub previous_version: u32,
    pub rollback_available: bool,
    pub has_migration: bool,
    pub migration_from_version: u32,
    pub migration_to_version: u32,
    pub migration_timestamp: u64,
    pub snapshot_count: u32,
    pub has_snapshot: bool,
    pub latest_snapshot_id: u64,
    pub latest_snapshot_version: u32,
}

/// Persisted migration result for audit and idempotency.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationState {
    pub from_version: u32,
    pub to_version: u32,
    pub migrated_at: u64,
    pub migration_hash: BytesN<32>,
}

/// Canonical read model for a multisig upgrade proposal.
///
/// Approval and execution status remain in [`MultiSig`], while upgrade-specific
/// metadata is stored in instance storage under the same stable `proposal_id`.
/// `proposer` is optional to preserve compatibility with older proposal rows
/// that predate explicit proposer storage.
///
/// # Expiry Semantics
/// `expiry == 0` means the proposal never expires. When `expiry > 0` and the
/// current ledger timestamp is at or past that value, the proposal is considered
/// expired and can no longer be approved or executed.
///
/// # Cancellation Semantics
/// `cancelled == true` means a signer has explicitly revoked the proposal.
/// Cancelled proposals can never be re-activated or executed.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeProposalRecord {
    /// Stable multisig proposal identifier returned by `propose_upgrade`.
    pub proposal_id: u64,
    /// Address that created the proposal, when explicitly recorded.
    pub proposer: Option<Address>,
    /// WASM hash that will be installed if the proposal executes.
    pub wasm_hash: BytesN<32>,
    /// Expiry ledger timestamp (seconds). `0` means no expiry.
    pub expiry: u64,
    /// Whether the proposal was explicitly cancelled by a signer.
    pub cancelled: bool,
}


/// [FIX-C01] Pre-committed migration hash for replay protection.
///
/// Admin must call `commit_migration(target_version, hash)` before calling
/// `migrate(target_version, hash)`. The commitment is verified during migration
/// to ensure the exact hash was pre-authorized by the admin in a separate tx.
///
/// # Replay Protection Flow
/// 1. Admin calls `commit_migration(3, hash_of_migration_data)` → stored on-chain
/// 2. Anyone can verify the commitment is live on-chain before execution
/// 3. Admin calls `migrate(3, hash_of_migration_data)` → hash verified against commitment
/// 4. Commitment is consumed (deleted) — cannot be replayed
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationCommitment {
    /// Target version this commitment authorizes
    pub target_version: u32,
    /// Hash committed by admin — must match hash passed to migrate()
    pub hash: BytesN<32>,
    /// Ledger timestamp when commitment was made
    pub committed_at: u64,
    /// Commitment expires after this timestamp (0 = no expiry)
    pub expires_at: u64,
}

/// [FIX-C02] Pending admin restore — two-step guard for snapshot-based admin changes.
///
/// When `restore_config_snapshot` would change the admin address, it creates
/// a pending restore instead of applying immediately. The NEW admin address
/// must then call `confirm_admin_restore(snapshot_id)` to finalize.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingAdminRestore {
    pub snapshot_id: u64,

    pub proposed_admin: Address,

    pub initiated_at: u64,
    /// Restore expires if not confirmed within this many seconds
    pub expires_at: u64,
}

/// Kind of contract deployed in the Grainlify ecosystem.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractKind {
    BountyEscrow,
    ProgramEscrow,
    SorobanEscrow,
    GrainlifyCore,
    ViewFacade,
    Other,
}

/// A single entry in the deployed-contract registry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeployedContract {
    /// On-chain address of the deployed contract.
    pub address: Address,
    /// Human-readable name of the contract (e.g. "bounty-escrow-v3").
    pub name: String,
    /// Role / type of the contract within the ecosystem.
    pub kind: ContractKind,
    /// Numeric version reported at registration time.
    pub version: u32,
    /// Ledger timestamp when the contract was registered.
    pub deployed_at: u64,
}

/// Liveness watchdog status — a single read-only view of the contract's
/// operational health, pause state, and maintenance mode.
///
/// Returned by `liveness_watchdog()`. All fields are safe to read without auth.
///
/// # Fields
/// * `paused`       — true when MultiSig pause is active (no payouts/upgrades)
/// * `read_only`    — true when read-only mode is set (mutations blocked)
/// * `healthy`      — true when monitoring invariants pass
/// * `last_ping_ts` — ledger timestamp of the last `ping_watchdog` call (0 if never)
/// * `version`      — current contract version number
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchdogStatus {
    pub paused: bool,
    pub read_only: bool,
    pub healthy: bool,
    pub last_ping_ts: u64,
    pub version: u32,
}

/// Liveness snapshot returned by `liveness_watchdog`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivenessStatus {
    pub is_paused: bool,
    pub is_read_only: bool,
    pub is_operational: bool,
    pub version: u32,
    pub admin_set: bool,
    pub timestamp: u64,
    pub schema_version: u32,
}

/// Storage keys for contract data.
///
/// # Keys
/// * `Admin` - Stores the administrator address (set once at initialization)
/// * `Version` - Stores the current contract version number
/// * `MigrationState` - Migration state tracking to prevent double migration
/// * `PreviousVersion` - Tracks previous version for rollback support
/// * `ChainId` - Stores the chain identifier for cross-network protection
/// * `NetworkId` - Stores the network identifier for environment-specific behavior
/// * `TimelockDelay` - Stores the timelock delay period for upgrade execution
/// * `UpgradeTimelock` - Stores the timelock start time for upgrade proposals
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
/// - Timelock delay prevents immediate execution after threshold approval
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    RegEntry(Symbol),
    Admin,

    /// Current version number (increments with upgrades)
    /// - Updated by migrate() and set_version()
    /// - Used to determine which migration functions to execute
    /// - Persists across all WASM upgrades
    Version,
  /// WASM hash stored per proposal (for multisig upgrades)
    UpgradeProposal(u64),

    /// Proposer recorded per upgrade proposal.
    /// - Added as a separate key to preserve compatibility with older
    ///   deployments that already store `UpgradeProposal(u64)` as a raw hash.
    /// - Uses the same stable proposal identifier returned by `propose_upgrade`.
    UpgradeProposalProposer(u64),

    /// Migration state tracking - prevents double migration
    /// - Set after successful migrate() call
    /// - Records from_version, to_version, timestamp, and migration_hash
    /// - Checked for idempotency in migrate() function
    /// - Persists across all WASM upgrades
    MigrationState,
    /// [FIX-C01] Pre-committed migration hash storage
    MigrationCommitment(u32), // keyed by target_version
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

    /// Read-only mode flag — blocks all state-mutating entrypoints
    ReadOnlyMode,

    /// Timelock delay period for upgrade execution (in seconds)
    /// - Default: 24 hours (86400 seconds) if not set
    /// - Can be adjusted by admin only
    /// - Applies to all upgrade proposals
    TimelockDelay,

    /// Timelock start time for upgrade proposals
    /// - Records when proposal threshold was met
    /// - Used to enforce delay before execution
    /// - proposal_id -> timestamp mapping
    UpgradeTimelock(u64),
    /// [FIX-C02] Pending admin restore awaiting new-admin confirmation
    PendingAdminRestore,
    /// Upgrade-safe schema version marker for liveness watchdog storage.
    /// Written on init_admin; increment when WatchdogStatus layout changes.
    LivenessSchemaVersion,
    /// Timestamp of the last successful ping_watchdog call.
    WatchdogLastPing,
}

// ============================================================================
// Monitoring Module
// ============================================================================

mod monitoring {
    use super::DataKey;
    use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Vec};

    const OPERATION_COUNT: &str = "op_count";
    const USER_COUNT: &str = "usr_count";
    const ERROR_COUNT: &str = "err_count";
    const USER_INDEX: &str = "usr_index";
    const LAST_OPERATION_TS: &str = "last_op_ts";

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct OperationMetric {
        pub operation: Symbol,
        pub caller: Address,
        pub timestamp: u64,
        pub success: bool,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct PerformanceMetric {
        pub function: Symbol,
        pub duration: u64,
        pub timestamp: u64,
    }

    #[contracttype]
    #[derive(Clone, Debug)]
    pub struct HealthStatus {
        pub is_healthy: bool,
        pub last_operation: u64,
        pub total_operations: u64,
        pub contract_version: String,
    }

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

    pub const MAX_TRACKED_FUNCTIONS: u32 = 50;
    pub const MAX_TRACKED_USERS: u32 = 64;

    fn get_counter(env: &Env, key: &str) -> u64 {
        env.storage()
            .persistent()
            .get(&Symbol::new(env, key))
            .unwrap_or(0)
    }

    fn set_counter(env: &Env, key: &str, value: u64) {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, key), &value);
    }

    fn get_tracked_users(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&Symbol::new(env, USER_INDEX))
            .unwrap_or(Vec::new(env))
    }

    fn track_unique_user(env: &Env, caller: &Address) {
        let mut users = get_tracked_users(env);
        for index in 0..users.len() {
            if users.get(index).unwrap() == *caller {
                return;
            }
        }
        if users.len() >= MAX_TRACKED_USERS {
            set_counter(env, USER_COUNT, MAX_TRACKED_USERS as u64);
            return;
        }
        users.push_back(caller.clone());
        env.storage()
            .persistent()
            .set(&Symbol::new(env, USER_INDEX), &users);
        set_counter(env, USER_COUNT, users.len().into());
    }

    /// [FIX-H03] Dynamic semver decoding — handles any version, not just hardcoded ones
    fn version_semver_string(env: &Env) -> String {
        let raw: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
        // Promote legacy single-digit versions (1,2,...) to major.0.0 encoding
        let encoded = if raw >= 10_000 { raw } else { raw.saturating_mul(10_000) };
        let major = encoded / 10_000;
        let minor = (encoded % 10_000) / 100;
        let patch = encoded % 100;

        // Build semver string without heap alloc
        let mut buf = [0u8; 12];
        let mut pos = 0usize;

        macro_rules! write_u32 {
            ($n:expr) => {
                let n: u32 = $n;
                if n >= 100 { buf[pos] = b'0' + (n / 100) as u8; pos += 1; }
                if n >= 10 { buf[pos] = b'0' + ((n % 100) / 10) as u8; pos += 1; }
                buf[pos] = b'0' + (n % 10) as u8; pos += 1;
            };
        }

        write_u32!(major);
        buf[pos] = b'.'; pos += 1;
        write_u32!(minor);
        buf[pos] = b'.'; pos += 1;
        write_u32!(patch);

        let s = core::str::from_utf8(&buf[..pos]).unwrap_or("0.0.0");
        String::from_str(env, s)
    }

    pub fn track_operation(env: &Env, operation: Symbol, caller: Address, success: bool) {
        let count = get_counter(env, OPERATION_COUNT);
        set_counter(env, OPERATION_COUNT, count.saturating_add(1));
        set_counter(env, LAST_OPERATION_TS, env.ledger().timestamp());
        track_unique_user(env, &caller);
        if !success {
            let err_count = get_counter(env, ERROR_COUNT);
            set_counter(env, ERROR_COUNT, err_count.saturating_add(1));
        }
        env.events().publish(
            (symbol_short!("metric"), symbol_short!("op")),
            OperationMetric { operation, caller, timestamp: env.ledger().timestamp(), success },
        );
    }

    pub fn emit_performance(env: &Env, function: Symbol, duration: u64) {
        let index_key = Symbol::new(env, "perf_index");
        let mut index: Vec<Symbol> = env
            .storage().persistent().get(&index_key).unwrap_or(Vec::new(env));

        let mut already_tracked = false;
        for i in 0..index.len() {
            if index.get(i).unwrap() == function { already_tracked = true; break; }
        }

        if !already_tracked {
            if index.len() >= MAX_TRACKED_FUNCTIONS {
                let oldest = index.get(0).unwrap();
                env.storage().persistent().remove(&(Symbol::new(env, "perf_cnt"), oldest.clone()));
                env.storage().persistent().remove(&(Symbol::new(env, "perf_time"), oldest.clone()));
                env.storage().persistent().remove(&(Symbol::new(env, "perf_last"), oldest.clone()));
                let mut trimmed = Vec::new(env);
                for i in 1..index.len() { trimmed.push_back(index.get(i).unwrap()); }
                index = trimmed;
            }
            index.push_back(function.clone());
            env.storage().persistent().set(&index_key, &index);
        }

        let count_key = (Symbol::new(env, "perf_cnt"), function.clone());
        let time_key = (Symbol::new(env, "perf_time"), function.clone());
        let last_key = (Symbol::new(env, "perf_last"), function.clone());
        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);
        let timestamp = env.ledger().timestamp();
        env.storage().persistent().set(&count_key, &count.saturating_add(1));
        env.storage().persistent().set(&time_key, &total.saturating_add(duration));
        env.storage().persistent().set(&last_key, &timestamp);
        env.events().publish(
            (symbol_short!("metric"), symbol_short!("perf")),
            PerformanceMetric { function, duration, timestamp },
        );
    }

    pub fn health_check(env: &Env) -> HealthStatus {
        let report = check_invariants(env);
        HealthStatus {
            is_healthy: report.healthy,
            last_operation: get_counter(env, LAST_OPERATION_TS),
            total_operations: report.operation_count,
            contract_version: version_semver_string(env), // [FIX-H03] now dynamic
        }
    }

    pub fn get_analytics(env: &Env) -> Analytics {
        let ops = get_counter(env, OPERATION_COUNT);
        let users = get_counter(env, USER_COUNT);
        let errors = get_counter(env, ERROR_COUNT);
        let error_rate = if ops > 0 {
            ((errors as u128 * 10000) / ops as u128) as u32
        } else { 0 };
        Analytics { operation_count: ops, unique_users: users, error_count: errors, error_rate }
    }

    pub fn get_state_snapshot(env: &Env) -> StateSnapshot {
        StateSnapshot {
            timestamp: env.ledger().timestamp(),
            total_operations: get_counter(env, OPERATION_COUNT),
            total_users: get_counter(env, USER_COUNT),
            total_errors: get_counter(env, ERROR_COUNT),
        }
    }

    pub fn get_performance_stats(env: &Env, function_name: Symbol) -> PerformanceStats {
        let count_key = (Symbol::new(env, "perf_cnt"), function_name.clone());
        let time_key = (Symbol::new(env, "perf_time"), function_name.clone());
        let last_key = (Symbol::new(env, "perf_last"), function_name.clone());
        let count: u64 = env.storage().persistent().get(&count_key).unwrap_or(0);
        let total: u64 = env.storage().persistent().get(&time_key).unwrap_or(0);
        let last: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);
        let avg = if count > 0 { total / count } else { 0 };
        PerformanceStats { function_name, call_count: count, total_time: total, avg_time: avg, last_called: last }
    }

    pub fn check_invariants(env: &Env) -> InvariantReport {
        let operation_count: u64 = get_counter(env, OPERATION_COUNT);
        let unique_users: u64 = get_counter(env, USER_COUNT);
        let error_count: u64 = get_counter(env, ERROR_COUNT);

        let metrics_sane = error_count <= operation_count
            && unique_users <= operation_count
            && (operation_count > 0 || (unique_users == 0 && error_count == 0));

        let admin_set = env.storage().instance().has(&DataKey::Admin);
        let version_opt: Option<u32> = env.storage().instance().get(&DataKey::Version);
        let version_set = version_opt.is_some();
        let version = version_opt.unwrap_or(0);
        let version_sane = version > 0;

        let previous_version_opt: Option<u32> = env.storage().instance().get(&DataKey::PreviousVersion);
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

        let config_sane = admin_set && version_set && version_sane && previous_version_sane && network_pair_sane;
        let mut violation_count: u32 = 0;
        if !admin_set { violation_count += 1; }
        if !version_set || !version_sane { violation_count += 1; }
        if !previous_version_sane { violation_count += 1; }
        if !network_pair_sane { violation_count += 1; }
        if error_count > operation_count { violation_count += 1; }
        if unique_users > operation_count { violation_count += 1; }
        if operation_count == 0 && (unique_users > 0 || error_count > 0) { violation_count += 1; }

        InvariantReport {
            healthy: config_sane && metrics_sane,
            config_sane, metrics_sane, admin_set, version_set, version,
            operation_count, unique_users, error_count, violation_count,
        }
    }

    pub fn verify_invariants(env: &Env) -> bool {
        let report = check_invariants(env);
        #[cfg(feature = "strict-mode")]
        {
            if !report.healthy {
                env.events().publish(
                    (symbol_short!("strict"), symbol_short!("inv_fail")),
                    report.violation_count,
                );
            }
        }
        report.healthy
    }
}

#[cfg(all(test, feature = "wasm_tests"))]
mod test_core_monitoring;
#[cfg(all(test, feature = "wasm_tests"))]
mod test_pseudo_randomness;
#[cfg(all(test, feature = "wasm_tests"))]
mod test_serialization_compatibility;
#[cfg(test)]
mod test_storage_layout;
#[cfg(all(test, feature = "wasm_tests"))]
mod test_version_helpers;
#[cfg(test)]
mod test_strict_mode;
#[cfg(test)]
mod test_contract_registry;
// ==================== END MONITORING MODULE ====================

#[cfg(feature = "contract")]
#[contract]
pub struct GrainlifyRegistry;
#[contractimpl]
impl GrainlifyContract {
    /// One-time initialization: set the admin and initial version. Requires `admin` auth.
    pub fn init_admin(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("{}", ContractError::AlreadyInitialized as u32);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
        env.storage().instance().set(&DataKey::ReadOnlyMode, &false);
        env.storage().instance().set(&DataKey::LivenessSchemaVersion, &LIVENESS_SCHEMA_VERSION);
        
        // Emit BuildInfo event for initialization tracking and auditing
        env.events().publish(
            (symbol_short!("init"), symbol_short!("build")),
            BuildInfoEvent {
                admin: admin.clone(),
                version: VERSION,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // ========================================================================
    // Timelock Execution (continued from propose/approve flow)
    // ========================================================================

    /// Execute a multisig-approved upgrade after the timelock delay has elapsed.
    pub fn execute_upgrade(env: Env, proposal_id: u64) {
        let start = env.ledger().timestamp();

        let timelock_start: u64 = env
            .storage()
            .instance()
            .get(&DataKey::UpgradeTimelock(proposal_id))
            .unwrap_or_else(|| panic!("Timelock not started - call approve_upgrade first"));

        let timelock_delay = Self::get_timelock_delay(env.clone());
        let current_time = env.ledger().timestamp();
        let elapsed = current_time.saturating_sub(timelock_start);

        if elapsed < timelock_delay {
            let remaining = timelock_delay.saturating_sub(elapsed);
            panic!("Timelock delay not met: {} seconds remaining", remaining);
        }
        e.storage().instance().set(&DataKey::Admin, &admin);
    }
    pub fn set_addr(e: Env, n: Symbol, a: Address) {
        let adm: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        adm.require_auth();
        e.storage().persistent().set(&DataKey::RegEntry(n), &a);
    }

    /// Single-admin upgrade path
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) {
        let start = env.ledger().timestamp();

        #[cfg(feature = "strict-mode")]
        {
            let report = monitoring::check_invariants(&env);
            strict_mode::strict_assert(report.healthy, "Strict mode: contract invariants unhealthy before upgrade");
            strict_mode::strict_emit(&env, symbol_short!("upgrade"), symbol_short!("pre_chk"));
        }

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("{}", ContractError::NotInitialized as u32));
        admin.require_auth();
        Self::require_not_read_only(&env);

        let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(1);
        env.storage().instance().set(&DataKey::PreviousVersion, &current_version);
        env.deployer().update_current_contract_wasm(new_wasm_hash.clone());

        // [FIX-L02] Consistent event shape with execute_upgrade
        env.events().publish(
            (symbol_short!("upgrade"), symbol_short!("wasm")),
            UpgradeEvent {
                new_wasm_hash,
                previous_version: current_version,
                timestamp: env.ledger().timestamp(),
            },
        );

        monitoring::track_operation(&env, symbol_short!("upgrade"), admin, true);
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("upgrade"), duration);
    }

    // ========================================================================
    // Timelock Management
    // ========================================================================

    pub fn get_timelock_delay(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::TimelockDelay)
            .unwrap_or(DEFAULT_TIMELOCK_DELAY)
    }

    /// [FIX-H02] Now enforces both minimum AND maximum to prevent bricking upgrades
    pub fn set_timelock_delay(env: Env, delay_seconds: u64) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        Self::require_not_read_only(&env);

        if delay_seconds < MIN_TIMELOCK_DELAY {
            panic!("Timelock delay must be at least 1 hour (3600 seconds)");
        }

        // [FIX-H02] Enforce maximum — prevents admin from bricking upgrade execution
        if delay_seconds > MAX_TIMELOCK_DELAY {
            panic!("Timelock delay cannot exceed 30 days (2592000 seconds)");
        }

        let old_delay = Self::get_timelock_delay(env.clone());
        env.storage().instance().set(&DataKey::TimelockDelay, &delay_seconds);

        env.events().publish(
            (symbol_short!("timelock"), symbol_short!("dly_chg")),
            (old_delay, delay_seconds),
        );
    }

    pub fn get_timelock_status(env: Env, proposal_id: u64) -> Option<u64> {
        if let Some(timelock_start) = env
            .storage()
            .instance()
            .get(&DataKey::UpgradeTimelock(proposal_id))
        {
            let timelock_delay = Self::get_timelock_delay(env.clone());
            let current_time = env.ledger().timestamp();
            let elapsed = current_time.saturating_sub(timelock_start);
            if elapsed >= timelock_delay { Some(0) } else { Some(timelock_delay.saturating_sub(elapsed)) }
        } else {
            None
        }
    }

    // ========================================================================
    // Version Management
    // ========================================================================

    pub fn get_version(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::Version).unwrap_or(0)
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    pub fn is_strict_mode(_env: Env) -> bool {
        strict_mode::is_enabled()
    }

    pub fn get_version_semver_string(env: Env) -> String {
        let encoded = Self::get_version_numeric_encoded(env.clone());
        let major = encoded / 10_000;
        let minor = (encoded % 10_000) / 100;
        let patch = encoded % 100;
        let mut buf = [0u8; 12];
        let mut pos = 0usize;

        macro_rules! write_u32 {
            ($n:expr) => {
                let n: u32 = $n;
                if n >= 100 { buf[pos] = b'0' + (n / 100) as u8; pos += 1; }
                if n >= 10 { buf[pos] = b'0' + ((n % 100) / 10) as u8; pos += 1; }
                buf[pos] = b'0' + (n % 10) as u8; pos += 1;
            };
        }
        write_u32!(major); buf[pos] = b'.'; pos += 1;
        write_u32!(minor); buf[pos] = b'.'; pos += 1;
        write_u32!(patch);

        let s = core::str::from_utf8(&buf[..pos]).unwrap_or("0.0.0");
        String::from_str(&env, s)
    }

    pub fn get_version_numeric_encoded(env: Env) -> u32 {
        let raw: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
        if raw >= 10_000 { raw } else { raw.saturating_mul(10_000) }
    }

    pub fn require_min_version(env: Env, min_numeric: u32) {
        let cur = Self::get_version_numeric_encoded(env.clone());
        if cur == 0 { panic!("{}", ContractError::NotInitialized as u32); }
        if cur < min_numeric { panic!("version_too_low"); }
    }

    pub fn set_version(env: Env, new_version: u32) {
        let start = env.ledger().timestamp();
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        Self::require_not_read_only(&env);
        env.storage().instance().set(&DataKey::Version, &new_version);
        monitoring::track_operation(&env, symbol_short!("set_ver"), admin, true);
        let duration = env.ledger().timestamp().saturating_sub(start);
        monitoring::emit_performance(&env, symbol_short!("set_ver"), duration);
    }

    // ========================================================================
    // Read-Only Mode
    // ========================================================================

    pub fn is_read_only(env: Env) -> bool {
        env.storage().instance().get(&DataKey::ReadOnlyMode).unwrap_or(false)
    }

    pub fn set_read_only_mode(env: Env, enabled: bool) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::ReadOnlyMode, &enabled);
        env.events().publish(
            (symbol_short!("ROModeChg"),),
            ReadOnlyModeEvent { enabled, admin, timestamp: env.ledger().timestamp() },
        );
    }

    fn require_not_read_only(env: &Env) {
        let read_only: bool = env.storage().instance().get(&DataKey::ReadOnlyMode).unwrap_or(false);
        if read_only { panic!("Read-only mode"); }
    }

    // ========================================================================
    // Config Snapshots
    // ========================================================================

    pub fn create_config_snapshot(env: Env) -> u64 {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        admin.require_auth();
        // [GUARDRAIL] Snapshots are state mutations — blocked in read-only mode
        Self::require_not_read_only(&env);

        let next_id: u64 = env.storage().instance()
            .get(&DataKey::SnapshotCounter).unwrap_or(0) + 1;

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

        env.storage().instance().set(&DataKey::ConfigSnapshot(next_id), &snapshot);

        let mut index: Vec<u64> = env.storage().instance()
            .get(&DataKey::SnapshotIndex).unwrap_or(Vec::new(&env));
        index.push_back(next_id);

        if index.len() > CONFIG_SNAPSHOT_LIMIT {
            let oldest_snapshot_id = index.get(0).unwrap();
            env.storage().instance().remove(&DataKey::ConfigSnapshot(oldest_snapshot_id));
            let mut trimmed = Vec::new(&env);
            for i in 1..index.len() { trimmed.push_back(index.get(i).unwrap()); }
            index = trimmed;
        }

        env.storage().instance().set(&DataKey::SnapshotIndex, &index);
        env.storage().instance().set(&DataKey::SnapshotCounter, &next_id);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("create")),
            (next_id, snapshot.timestamp),
        );
        next_id
    }

    pub fn list_config_snapshots(env: Env) -> Vec<CoreConfigSnapshot> {
        let index: Vec<u64> = env.storage().instance()
            .get(&DataKey::SnapshotIndex).unwrap_or(Vec::new(&env));
        let mut snapshots: Vec<CoreConfigSnapshot> = Vec::new(&env);
        for i in 0..index.len() {
            let snapshot_id = index.get(i).unwrap();
            if let Some(snapshot) = env.storage().instance()
                .get::<DataKey, CoreConfigSnapshot>(&DataKey::ConfigSnapshot(snapshot_id))
            {
                snapshots.push_back(snapshot);
            }
        }
        snapshots
    }

    pub fn get_config_snapshot(env: Env, snapshot_id: u64) -> Option<CoreConfigSnapshot> {
        env.storage().instance().get(&DataKey::ConfigSnapshot(snapshot_id))
    }

    pub fn get_latest_config_snapshot(env: Env) -> Option<CoreConfigSnapshot> {
        let index: Vec<u64> = env.storage().instance()
            .get(&DataKey::SnapshotIndex).unwrap_or(Vec::new(&env));
        if index.is_empty() { return None; }
        let latest_id = index.get(index.len() - 1).unwrap();
        env.storage().instance().get(&DataKey::ConfigSnapshot(latest_id))
    }

    pub fn get_snapshot_count(env: Env) -> u32 {
        let index: Vec<u64> = env.storage().instance()
            .get(&DataKey::SnapshotIndex).unwrap_or(Vec::new(&env));
        index.len()
    }

    pub fn compare_snapshots(env: Env, from_id: u64, to_id: u64) -> SnapshotDiff {
        let from: CoreConfigSnapshot = env.storage().instance()
            .get(&DataKey::ConfigSnapshot(from_id))
            .unwrap_or_else(|| panic!("Snapshot not found: from_id"));
        let to: CoreConfigSnapshot = env.storage().instance()
            .get(&DataKey::ConfigSnapshot(to_id))
            .unwrap_or_else(|| panic!("Snapshot not found: to_id"));
        SnapshotDiff {
            from_id, to_id,
            admin_changed: from.admin != to.admin,
            version_changed: from.version != to.version,
            previous_version_changed: from.previous_version != to.previous_version,
            multisig_threshold_changed: from.multisig_threshold != to.multisig_threshold,
            multisig_signers_changed: from.multisig_signers != to.multisig_signers,
            from_version: from.version,
            to_version: to.version,
        }
    }

    /// [FIX-C02] Restore now uses two-step process when admin address changes.
    ///
    /// If the snapshot would change the admin address, a `PendingAdminRestore`
    /// is created instead of applying immediately. The new admin address must
    /// call `confirm_admin_restore()` to finalize.
    ///
    /// If the snapshot does NOT change the admin, restore applies immediately
    /// (same behavior as before).
    pub fn restore_config_snapshot(env: Env, snapshot_id: u64) {
        let admin: Address = env.storage().instance()
            .get(&DataKey::Admin).expect("Admin not set");
        admin.require_auth();
        // [GUARDRAIL] Restores mutate state — blocked in read-only mode
        Self::require_not_read_only(&env);

        // [FIX-M02] Explicit error when snapshot is pruned
        let snapshot: CoreConfigSnapshot = env.storage().instance()
            .get(&DataKey::ConfigSnapshot(snapshot_id))
            .unwrap_or_else(|| panic!("Snapshot not found or has been pruned"));

        let current_admin: Option<Address> = env.storage().instance().get(&DataKey::Admin);

        // [FIX-C02] Detect if restore would change admin — if so, require two-step confirmation
        let admin_would_change = snapshot.admin != current_admin;

        if admin_would_change {
            // Create pending restore — new admin must confirm
            let pending = PendingAdminRestore {
                snapshot_id,
                proposed_admin: snapshot.admin.clone().expect("Snapshot has no admin to restore"),
                initiated_at: env.ledger().timestamp(),
                expires_at: env.ledger().timestamp().saturating_add(DEFAULT_TIMELOCK_DELAY),
            };
            env.storage().instance().set(&DataKey::PendingAdminRestore, &pending);

            env.events().publish(
                (symbol_short!("cfg_snap"), symbol_short!("adm_pnd")),
                (snapshot_id, pending.proposed_admin, pending.expires_at),
            );
            // Return early — do not apply yet
            return;
        }

        // Admin unchanged — apply restore immediately
        Self::apply_snapshot_restore(&env, &snapshot);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("restore")),
            (snapshot_id, env.ledger().timestamp()),
        );
    }

    /// [FIX-C02] The proposed new admin confirms an admin-changing snapshot restore.
    ///
    /// Only the address that would BECOME the new admin can confirm this.
    /// This ensures a compromised old key cannot silently transfer control.
    pub fn confirm_admin_restore(env: Env, snapshot_id: u64) {
        let pending: PendingAdminRestore = env.storage().instance()
            .get(&DataKey::PendingAdminRestore)
            .unwrap_or_else(|| panic!("No pending admin restore found"));

        if pending.snapshot_id != snapshot_id {
            panic!("Snapshot ID does not match pending restore");
        }

        // The proposed new admin must authorize this
        pending.proposed_admin.require_auth();

        // [GUARDRAIL] Confirm is a state mutation — blocked in read-only mode
        Self::require_not_read_only(&env);

        // Check expiry
        if env.ledger().timestamp() > pending.expires_at {
            env.storage().instance().remove(&DataKey::PendingAdminRestore);
            panic!("Pending admin restore has expired");
        }

        let snapshot: CoreConfigSnapshot = env.storage().instance()
            .get(&DataKey::ConfigSnapshot(snapshot_id))
            .unwrap_or_else(|| panic!("Snapshot not found"));

        Self::apply_snapshot_restore(&env, &snapshot);

        env.storage().instance().remove(&DataKey::PendingAdminRestore);

        env.events().publish(
            (symbol_short!("cfg_snap"), symbol_short!("adm_conf")),
            (snapshot_id, env.ledger().timestamp()),
        );
    }

    /// Internal: applies snapshot state to storage
    fn apply_snapshot_restore(env: &Env, snapshot: &CoreConfigSnapshot) {
        if let Some(ref snapshot_admin) = snapshot.admin {
            env.storage().instance().set(&DataKey::Admin, snapshot_admin);
        } else {
            env.storage().instance().remove(&DataKey::Admin);
        }

        env.storage().instance().set(&DataKey::Version, &snapshot.version);

        match snapshot.previous_version {
            Some(prev) => env.storage().instance().set(&DataKey::PreviousVersion, &prev),
            None => env.storage().instance().remove(&DataKey::PreviousVersion),
        }

        if snapshot.multisig_threshold > 0 {
            let config = multisig::MultiSigConfig {
                signers: snapshot.multisig_signers.clone(),
                threshold: snapshot.multisig_threshold,
            };
            MultiSig::set_config(env, config);
        } else {
            MultiSig::clear_config(env);
        }
    }

    /// [FIX-L04] Returns None on inconsistency instead of panicking — view fn safety
    pub fn get_rollback_info(env: Env) -> RollbackInfo {
        let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(0);
        let previous_version: u32 = env.storage().instance().get(&DataKey::PreviousVersion).unwrap_or(0);
        let rollback_available = previous_version > 0;

        let migration_state: Option<MigrationState> = env.storage().instance().get(&DataKey::MigrationState);
        let has_migration = migration_state.is_some();
        let migration_from_version = migration_state.as_ref().map(|m| m.from_version).unwrap_or(0);
        let migration_to_version = migration_state.as_ref().map(|m| m.to_version).unwrap_or(0);
        let migration_timestamp = migration_state.as_ref().map(|m| m.migrated_at).unwrap_or(0);

        let index: Vec<u64> = env.storage().instance()
            .get(&DataKey::SnapshotIndex).unwrap_or(Vec::new(&env));
        let snapshot_count = index.len();
        let has_snapshot = snapshot_count > 0;

        // [FIX-L04] Use Option pattern instead of panic on inconsistency
        let (latest_snapshot_id, latest_snapshot_version) = if has_snapshot {
            let latest_id = index.get(snapshot_count - 1).unwrap();
            let snap: Option<CoreConfigSnapshot> = env.storage().instance()
                .get(&DataKey::ConfigSnapshot(latest_id));
            match snap {
                Some(s) => (latest_id, s.version),
                None => (0u64, 0u32), // Inconsistency: return safe defaults
            }
        } else {
            (0u64, 0u32)
        };

        RollbackInfo {
            current_version, previous_version, rollback_available,
            has_migration, migration_from_version, migration_to_version,
            migration_timestamp, snapshot_count, has_snapshot,
            latest_snapshot_id, latest_snapshot_version,
        }
    }

    // ========================================================================
    // Network Configuration
    // ========================================================================

    pub fn get_chain_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::ChainId)
    }

    pub fn get_network_id(env: Env) -> Option<String> {
        env.storage().instance().get(&DataKey::NetworkId)
    }

    pub fn get_network_info(env: Env) -> (Option<String>, Option<String>) {
        let chain_id = env.storage().instance().get(&DataKey::ChainId);
        let network_id = env.storage().instance().get(&DataKey::NetworkId);
        (chain_id, network_id)
    }

    // ========================================================================
    // Storage Layout Verification
    // ========================================================================

    pub fn verify_storage_layout(env: Env) -> bool {
        let admin_ok = env.storage().instance().has(&DataKey::Admin)
            && env.storage().instance().get::<_, Address>(&DataKey::Admin).is_some();

        let version_ok = env.storage().instance().has(&DataKey::Version)
            && env.storage().instance().get::<_, u32>(&DataKey::Version).is_some();

        let migration_ok = if env.storage().instance().has(&DataKey::MigrationState) {
            // [FIX-L03] Also verify MigrationState schema is readable
            env.storage().instance()
                .get::<_, crate::MigrationState>(&DataKey::MigrationState)
                .is_some()
        } else {
            true
        };

        admin_ok && version_ok && migration_ok
    }

    // ========================================================================
    // Monitoring & Analytics
    // ========================================================================

    pub fn health_check(env: Env) -> monitoring::HealthStatus {
        monitoring::health_check(&env)
    }

    pub fn get_analytics(env: Env) -> monitoring::Analytics {
        monitoring::get_analytics(&env)
    }

    pub fn get_state_snapshot(env: Env) -> monitoring::StateSnapshot {
        monitoring::get_state_snapshot(&env)
    }

    pub fn get_performance_stats(env: Env, function_name: Symbol) -> monitoring::PerformanceStats {
        monitoring::get_performance_stats(&env, function_name)
    }

    pub fn check_invariants(env: Env) -> monitoring::InvariantReport {
        monitoring::check_invariants(&env)
    }

    pub fn verify_invariants(env: Env) -> bool {
        monitoring::verify_invariants(&env)
    }

    // ========================================================================
    // Emergency Controls
    // ========================================================================

    pub fn pause(env: Env, signer: Address) {
        MultiSig::pause(&env, signer);
    }

    pub fn unpause(env: Env, signer: Address) {
        MultiSig::unpause(&env, signer);
    }

    pub fn is_paused(env: Env) -> bool {
        MultiSig::is_contract_paused(&env)
    }

    /// Unified liveness watchdog view — no auth required, never panics.
    ///
    /// Returns a `LivenessStatus` snapshot combining pause state, read-only
    /// mode, monitoring health, last-ping timestamp, version, and admin
    /// presence.  Designed for polling by monitoring agents, circuit breakers,
    /// and dashboards.
    ///
    /// # Upgrade Safety
    /// `schema_version` reflects `LivenessSchemaVersion` written at `init_admin`.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn liveness_watchdog(env: Env) -> LivenessStatus {
        let is_paused = MultiSig::is_contract_paused(&env);
        let is_read_only: bool = env
            .storage()
            .instance()
            .get(&DataKey::ReadOnlyMode)
            .unwrap_or(false);
        let version: u32 = env
            .storage()
            .instance()
            .get(&DataKey::Version)
            .unwrap_or(0);
        let healthy = monitoring::check_invariants(&env).healthy;
        let last_ping_ts: u64 = env
            .storage()
            .instance()
            .get(&DataKey::WatchdogLastPing)
            .unwrap_or(0);
        LivenessStatus {
            is_paused,
            is_read_only,
            is_operational: !is_paused && !is_read_only,
            admin_set: env.storage().instance().has(&DataKey::Admin),
            schema_version: env
                .storage()
                .instance()
                .get(&DataKey::LivenessSchemaVersion)
                .unwrap_or(0),
            timestamp: env.ledger().timestamp(),
            paused: is_paused,
            read_only: is_read_only,
            healthy,
            last_ping_ts,
            version,
        }
    }

    /// Returns the liveness schema version written at init.
    /// Returns `0` on legacy deployments where the marker was never written.
    pub fn get_liveness_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::LivenessSchemaVersion)
            .unwrap_or(0)
    }

    pub fn can_execute(env: Env, proposal_id: u64) -> bool {
        MultiSig::can_execute(&env, proposal_id)
    }

    // ========================================================================
    // Liveness Watchdog
    // ========================================================================

    /// Admin: record a liveness ping — updates `WatchdogLastPing` timestamp.
    ///
    /// Allows off-chain monitors to prove the contract is reachable and the
    /// admin key is live. Blocked when read-only mode is active.
    ///
    /// # Authorization
    /// Requires admin signature.
    pub fn ping_watchdog(env: Env) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("{}", ContractError::NotInitialized as u32));
        admin.require_auth();
        Self::require_not_read_only(&env);
        let ts = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::WatchdogLastPing, &ts);
        env.events().publish(
            (symbol_short!("watchdog"), symbol_short!("ping")),
            (admin, ts),
        );
    }

    /// Returns the liveness schema version written at `init_admin`.
    /// Returns `0` on legacy deployments where the marker was never written.
    /// Increment `LivenessSchemaVersion` in `init_admin` whenever `WatchdogStatus` layout changes.
    pub fn get_liveness_schema_version(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::LivenessSchemaVersion)
            .unwrap_or(0)
    }

    // ========================================================================
    // Migration State Queries
    // ========================================================================

    pub fn get_migration_state(env: Env) -> Option<MigrationState> {
        if env.storage().instance().has(&DataKey::MigrationState) {
            Some(env.storage().instance().get(&DataKey::MigrationState).unwrap())
        } else {
            None
        }
    }

    pub fn get_previous_version(env: Env) -> Option<u32> {
        if env.storage().instance().has(&DataKey::PreviousVersion) {
            Some(env.storage().instance().get(&DataKey::PreviousVersion).unwrap())
        } else {
            None
        }
    }

    // ========================================================================
    // Multisig Initialization
    // ========================================================================

    /// Initialize with multisig governance (alternative to init_admin).
    /// Requires at least one signer and a valid threshold.
    pub fn init(env: Env, signers: Vec<Address>, threshold: u32) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("Already initialized");
        }
        MultiSig::init(&env, signers, threshold);
        env.storage().instance().set(&DataKey::Version, &VERSION);
        env.storage().instance().set(&DataKey::ReadOnlyMode, &false);
    }

    /// Initialize with admin, chain_id, and network_id (network-aware init).
    pub fn init_with_network(env: Env, admin: Address, chain_id: String, network_id: String) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
        env.storage().instance().set(&DataKey::ReadOnlyMode, &false);
        env.storage().instance().set(&DataKey::ChainId, &chain_id);
        env.storage().instance().set(&DataKey::NetworkId, &network_id);
    }

    /// Initialize with governance configuration.
    pub fn init_governance(env: Env, admin: Address, config: GovernanceConfig) {
        if env.storage().instance().has(&DataKey::Version) {
            panic!("Already initialized");
        }
        admin.require_auth();
        if config.quorum_percentage == 0 || config.quorum_percentage > 10000 {
            panic!("Invalid quorum percentage");
        }
        if config.approval_threshold < 5000 || config.approval_threshold > 10000 {
            panic!("Invalid approval threshold");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Version, &VERSION);
        env.storage().instance().set(&DataKey::ReadOnlyMode, &false);
        env.storage().instance().set(&governance::GOVERNANCE_CONFIG, &config);
        env.storage().instance().set(&governance::PROPOSAL_COUNT, &0u32);
    }

    // ========================================================================
    // Multisig Upgrade Proposal Flow
    // ========================================================================

    /// Propose a WASM upgrade via multisig. Returns the stable proposal ID.
    /// `expiry` is a ledger timestamp after which the proposal cannot be approved
    /// or executed (0 = no expiry).
    pub fn propose_upgrade(env: Env, proposer: Address, wasm_hash: BytesN<32>, expiry: u64) -> u64 {
        Self::require_not_paused(&env);
        Self::require_not_read_only(&env);
        let proposal_id = MultiSig::propose(&env, proposer.clone(), expiry);
        env.storage().instance().set(&DataKey::UpgradeProposal(proposal_id), &wasm_hash);
        env.storage().instance().set(&DataKey::UpgradeProposalProposer(proposal_id), &proposer);
        proposal_id
    }

    /// Approve a pending upgrade proposal. Starts the timelock when threshold is met.
    pub fn approve_upgrade(env: Env, proposal_id: u64, signer: Address) {
        Self::require_not_paused(&env);
        MultiSig::approve(&env, proposal_id, signer);
        // Start timelock if threshold is now met and not already started
        if MultiSig::can_execute(&env, proposal_id)
            && !env.storage().instance().has(&DataKey::UpgradeTimelock(proposal_id))
        {
            let now = env.ledger().timestamp();
            env.storage().instance().set(&DataKey::UpgradeTimelock(proposal_id), &now);
            env.events().publish(
                (Symbol::new(&env, "timelock"), Symbol::new(&env, "started")),
                (proposal_id, now),
            );
        }
    }

    /// Cancel a pending upgrade proposal. Any signer may cancel.
    pub fn cancel_upgrade(env: Env, proposal_id: u64, canceller: Address) {
        MultiSig::cancel(&env, proposal_id, canceller);
        env.storage().instance().remove(&DataKey::UpgradeTimelock(proposal_id));
    }

    /// Return the upgrade proposal record for a given proposal ID, or None.
    pub fn get_upgrade_proposal(env: Env, proposal_id: u64) -> Option<UpgradeProposalRecord> {
        Self::load_upgrade_proposal(&env, proposal_id)
    }

    // ========================================================================
    // Migration
    // ========================================================================

    /// Pre-commit a migration hash for replay protection.
    /// Must be called before `migrate()` with the same target_version and hash.
    pub fn commit_migration(env: Env, target_version: u32, hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("{}", ContractError::NotInitialized as u32));
        admin.require_auth();
        Self::require_not_read_only(&env);
        let commitment = MigrationCommitment {
            target_version,
            hash,
            committed_at: env.ledger().timestamp(),
            expires_at: 0,
        };
        env.storage().instance().set(&DataKey::MigrationCommitment(target_version), &commitment);
        env.events().publish(
            (symbol_short!("migrate"), symbol_short!("commit")),
            (target_version, env.ledger().timestamp()),
        );
    }

    /// Execute a state migration to `target_version`.
    ///
    /// Requires a prior `commit_migration` call with the same hash (replay protection).
    /// Idempotent: migrating to the same version twice is a no-op after the first call.
    pub fn migrate(env: Env, target_version: u32, migration_hash: BytesN<32>) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("{}", ContractError::NotInitialized as u32));
        admin.require_auth();
        Self::require_not_read_only(&env);

        // Idempotency: skip if already migrated to this version
        if let Some(state) = env.storage().instance().get::<_, MigrationState>(&DataKey::MigrationState) {
            if state.to_version == target_version {
                return;
            }
        }

        // [FIX-C01] Verify commitment exists and hash matches
        let commitment: MigrationCommitment = env.storage().instance()
            .get(&DataKey::MigrationCommitment(target_version))
            .unwrap_or_else(|| panic!("{}", ContractError::MigrationCommitmentNotFound as u32));

        if commitment.hash != migration_hash {
            panic!("{}", ContractError::MigrationHashMismatch as u32);
        }

        let current_version: u32 = env.storage().instance().get(&DataKey::Version).unwrap_or(1);

        // Run version-specific migration logic
        if current_version == 1 && target_version == 2 {
            migrate_v1_to_v2(&env);
        }

        let state = MigrationState {
            from_version: current_version,
            to_version: target_version,
            migrated_at: env.ledger().timestamp(),
            migration_hash: migration_hash.clone(),
        };
        env.storage().instance().set(&DataKey::MigrationState, &state);
        env.storage().instance().set(&DataKey::Version, &target_version);

        // Consume commitment (replay protection)
        env.storage().instance().remove(&DataKey::MigrationCommitment(target_version));

        env.events().publish(
            (symbol_short!("migrate"), symbol_short!("done")),
            (current_version, target_version, env.ledger().timestamp()),
        );

        monitoring::track_operation(&env, symbol_short!("migrate"), admin, true);
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    fn require_not_paused(env: &Env) {
        if MultiSig::is_contract_paused(env) {
            panic!("Contract is paused");
        }
    }
}

// ============================================================================
// Trait Conformance
// ============================================================================

pub mod traits {
    use soroban_sdk::{Env, String};

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
        GrainlifyContract::set_version(env.clone(), new_version);
        Ok(())
    }
}
mod test;
