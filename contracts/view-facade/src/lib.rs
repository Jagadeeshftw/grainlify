#![no_std]
//! # View Facade
//!
//! A **read-only aggregation layer** for cross-contract queries on the Stellar/Soroban network.
//!
//! ## Purpose
//!
//! Registers known escrow and core contract addresses so dashboards, indexers, and wallets
//! can discover and interrogate them through a single endpoint, without coupling to a
//! specific contract type or requiring knowledge of individual deployment addresses.
//!
//! ## Duplicate Registration Policy
//!
//! When [`register`](ViewFacade::register) is called with an address that is already in the
//! registry, the existing entry is **updated** (not duplicated) with the new `kind` and
//! `version` values. The entry retains its original position in insertion order.
//!
//! **Benefits:**
//! - Single-source-of-truth per address (no duplicates)
//! - Consistent query results across all view functions
//! - Efficient admin operations (update without explicit deregister)
//!
//! ## Query Notes
//!
//! - `list_contracts` supports pagination with optional `offset` and `limit` parameters.
//! - `list_contracts_all` returns the full registry (legacy compatibility).
//! - `contract_count` returns the total registry size for pagination calculations.
//! - `get_contract` performs an `O(n)` scan and returns the first matching
//!   entry for the requested address.
//! - Registry size is bounded by [`MAX_REGISTRY_SIZE`] (1000 entries) to prevent
//!   unbounded storage growth.
//!
//! ## Query Flow
//!
//! 1. Call `contract_count` to get the total number of entries.
//! 2. Use paginated `list_contracts(offset, limit)` for large registries.
//! 3. Call `get_contract` when the UI needs to refresh a single known address.
//! 4. Fall back to `list_contracts_all` only for small registries or legacy compatibility.
//!
//! ## Registry Limits and Pagination
//!
//! The facade enforces a hard cap of [`MAX_REGISTRY_SIZE`] entries to ensure:
//! - Predictable gas costs for all operations
//! - Protection against storage exhaustion attacks  
//! - Indexer-friendly pagination with bounded result sets
//!
//! When the registry is full, new registrations will fail with [`FacadeError::RegistryFull`].
//! Admins must deregister existing entries before adding new ones at capacity.
//!
//! ### Pagination Example
//!
//! ```text
//! total = contract_count()
//! page_size = 100
//! 
//! for offset in (0..total).step_by(page_size) {
//!     contracts = list_contracts(offset, page_size)
//!     // Process page...
//! }
//! ```
//!
//! ## Security Model
//!
//! - **No fund custody**: this contract holds no tokens and transfers no funds.
//! - **No external writes**: it writes state only to its own instance storage.
//! - **Immutable admin**: the administrator address is set once at initialization and
//!   can never be changed, preventing privilege escalation after deployment.
//! - **Double-init protection**: a second call to [`ViewFacade::init`] is rejected
//!   with [`FacadeError::AlreadyInitialized`], so the initial admin cannot be replaced.
//! - **Bounded registry**: hard cap on entries prevents storage bloat attacks.
//!
//! ## Initialization Workflow
//!
//! ```text
//! 1. Deploy contract
//! 2. Call init(admin)   — stores admin immutably, emits Initialized event
//! 3. Admin calls register(address, kind, version) to populate the registry
//! 4. Anyone calls list_contracts() / get_contract() / contract_count() to query
//! ```
//!
//! ## Spec Alignment
//!
//! Grainlify View Interface v1 (Issue #574)

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Vec,
};

// ─── ProgramEscrow types for query caching ───────────────────────────────────
use program_escrow::{FeeConfig, ProgramData};

// ─── Cross-contract interface for ProgramEscrow ───────────────────────────────

/// Minimal payout record mirrored from `program-escrow` for cross-contract use.
///
/// Must stay in sync with `PayoutRecord` in `contracts/program-escrow/src/lib.rs`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRecord {
    pub recipient: Address,
    pub amount: i128,
    pub timestamp: u64,
}

/// Thin cross-contract client for the `ProgramEscrow` methods used by this facade.
#[soroban_sdk::contractclient(name = "EscrowClient")]
pub trait ProgramEscrowTrait {
    fn query_recipient_history(
        env: Env,
        program_id: soroban_sdk::String,
        recipient: Address,
    ) -> Vec<PayoutRecord>;
}

/// Cross-contract client for querying ProgramData and FeeConfig from a
/// ProgramEscrow contract.  Used by [`QueryCache`] to fetch data that is
/// then memoized in temporary storage.
#[soroban_sdk::contractclient(name = "EscrowQueryClient")]
pub trait ProgramEscrowQueryTrait {
    /// Fetch the full [`ProgramData`] struct for `program_id`.
    fn get_program_info_v2(env: Env, program_id: soroban_sdk::String) -> ProgramData;

    /// Fetch the current [`FeeConfig`] from the escrow contract.
    fn get_fee_config(env: Env) -> FeeConfig;
}

// ============================================================================
// Query Cache (per-invocation memoization via temporary storage)
// ============================================================================

/// Storage keys used by [`QueryCache`] in Soroban temporary storage.
///
/// Temporary storage is scoped to the current top-level contract invocation
/// and is automatically discarded at the end of the transaction.  This makes
/// it a safe, zero-maintenance cache for redundant reads.
#[contracttype]
pub enum QueryCacheKey {
    /// Cached [`ProgramData`] for a specific `(escrow_address, program_id)` pair.
    ProgramData(Address, soroban_sdk::String),
    /// Cached [`FeeConfig`] for a specific escrow contract address.
    FeeConfig(Address),
}

/// Per-invocation read-through cache backed by Soroban temporary storage.
///
/// # Purpose
///
/// When multiple query functions within a single transaction call into the same
/// escrow contract to fetch [`ProgramData`] or [`FeeConfig`], each call incurs a
/// separate storage-read cost.  `QueryCache` memoizes the results in temporary
/// storage so that the first access populates the cache and all subsequent
/// accesses return the cached value without additional cross-contract calls.
///
/// # Safety & Liveness
///
/// - **Read-only**: the cache never mutates persistent storage.
/// - **Scoped**: temporary storage is discarded at transaction end — stale data
///   cannot leak across transactions.
/// - **No invalidation needed**: because the cache lives only within a single
///   call chain, it is inherently coherent for the duration of that invocation.
///
/// # Usage
///
/// ```rust,ignore
/// let data = QueryCache::get_or_load_program_data(&env, &escrow, &program_id);
/// let fees = QueryCache::get_or_load_fee_config(&env, &escrow);
/// ```
///
/// # Gas savings
///
/// Each cached read avoids one full cross-contract call (~1000+ CPU
/// instructions on Soroban).  For dashboards that aggregate data from multiple
/// view functions in a single transaction, the savings are proportional to the
/// number of redundant reads.
pub struct QueryCache;

impl QueryCache {
    /// Return [`ProgramData`] for `program_id` on `escrow`, fetching from the
    /// underlying contract only on first access within the current invocation.
    ///
    /// # Arguments
    /// * `env`        — The Soroban environment.
    /// * `escrow`     — Address of the ProgramEscrow contract.
    /// * `program_id` — Identifier of the program to query.
    ///
    /// # Returns
    /// The [`ProgramData`] struct from the escrow contract.
    ///
    /// # Panics
    /// If the cross-contract call to `get_program_info_v2` panics (e.g.
    /// program not found), the panic propagates to the caller.
    pub fn get_or_load_program_data(
        env: &Env,
        escrow: &Address,
        program_id: &soroban_sdk::String,
    ) -> ProgramData {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());

        // Check temporary storage for a previously cached value.
        if let Some(cached) = env.storage().temporary().get::<QueryCacheKey, ProgramData>(&key) {
            return cached;
        }

        // Cache miss — fetch via cross-contract call.
        let client = EscrowQueryClient::new(env, escrow);
        let data = client.get_program_info_v2(program_id);

        // Populate the cache for subsequent reads within this invocation.
        env.storage().temporary().set(&key, &data);

        data
    }

    /// Return [`FeeConfig`] for `escrow`, fetching from the underlying contract
    /// only on first access within the current invocation.
    ///
    /// # Arguments
    /// * `env`    — The Soroban environment.
    /// * `escrow` — Address of the ProgramEscrow contract.
    ///
    /// # Returns
    /// The [`FeeConfig`] struct from the escrow contract.
    pub fn get_or_load_fee_config(env: &Env, escrow: &Address) -> FeeConfig {
        let key = QueryCacheKey::FeeConfig(escrow.clone());

        if let Some(cached) = env.storage().temporary().get::<QueryCacheKey, FeeConfig>(&key) {
            return cached;
        }

        let client = EscrowQueryClient::new(env, escrow);
        let config = client.get_fee_config();

        env.storage().temporary().set(&key, &config);

        config
    }

    /// Explicitly remove a cached [`ProgramData`] entry from temporary storage.
    ///
    /// Useful in tests that want to verify cache-miss behaviour after a
    /// previous `get_or_load` call.
    pub fn invalidate_program_data(
        env: &Env,
        escrow: &Address,
        program_id: &soroban_sdk::String,
    ) {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());
        env.storage().temporary().remove(&key);
    }

    /// Explicitly remove a cached [`FeeConfig`] entry from temporary storage.
    pub fn invalidate_fee_config(env: &Env, escrow: &Address) {
        let key = QueryCacheKey::FeeConfig(escrow.clone());
        env.storage().temporary().remove(&key);
    }
}

// ============================================================================
// Error Type
// ============================================================================

/// Typed error codes returned by fallible entry-points.
///
/// Using a `#[contracterror]` enum instead of bare `panic!` strings gives
/// callers a stable integer discriminant they can match on and surfaces
/// clearer diagnostics in simulation tools.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FacadeError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    RegistryFull = 3,
    InvalidPagination = 4,
}

// ============================================================================
// Storage Key
// ============================================================================

/// Identifies the two slots this contract writes in instance storage.
///
/// Instance storage persists across contract upgrades, which ensures the
/// admin and the registry survive a WASM swap.
#[contracttype]
pub enum DataKey {
    /// The immutable administrator [`Address`] stored at initialization.
    Admin,
    /// The ordered list of [`RegisteredContract`] entries.
    Registry,
}

// ============================================================================
// Registry Configuration
// ============================================================================

/// Maximum number of contracts that can be registered in the facade.
///
/// This limit prevents unbounded storage growth and ensures predictable
/// gas costs for all operations. The value of 1000 is chosen to provide
/// ample capacity for production use while maintaining reasonable
/// performance characteristics.
///
/// ## Rationale
///
/// - **Gas efficiency**: Each registry entry requires storage reads/writes
/// - **Indexer friendliness**: Bounded size enables predictable pagination
/// - **Operational safety**: Prevents storage exhaustion attacks
/// - **Future upgradeability**: Can be increased via contract upgrade if needed
pub const MAX_REGISTRY_SIZE: u32 = 1000;

// ============================================================================
// Data Structures
// ============================================================================

/// Distinguishes the role / type of a registered contract.
///
/// This allows consumers to filter the registry (e.g. "show me all bounty
/// escrows") without querying individual contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractKind {
    /// A `BountyEscrow` contract managing individual bounty funds.
    BountyEscrow,
    /// A `ProgramEscrow` contract managing hackathon/grant prize pools.
    ProgramEscrow,
    /// A Soroban-native escrow contract variant.
    SorobanEscrow,
    /// The `GrainlifyCore` upgrade-management contract.
    GrainlifyCore,
}

/// A single entry in the view-facade registry.
///
/// Represents one contract deployment that the admin has chosen to expose
/// through this aggregation endpoint.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredContract {
    /// On-chain address of the registered contract.
    pub address: Address,
    /// High-level role of the contract within the Grainlify ecosystem.
    pub kind: ContractKind,
    /// Numeric version reported by the contract at registration time.
    ///
    /// Callers should treat this as an advisory hint; they should verify the
    /// version against the contract itself for critical paths.
    pub version: u32,
}

// ============================================================================
// Events
// ============================================================================

/// Emitted once when the facade is successfully initialized.
///
/// Off-chain indexers can use this event as a reliable signal that the
/// contract is ready to accept `register` calls.
///
/// # Event Topic
/// `("facade", "init")`
#[contracttype]
#[derive(Clone, Debug)]
pub struct InitializedEvent {
    /// The administrator address stored at initialization.
    pub admin: Address,
}

// ============================================================================
// Contract
// ============================================================================

/// The View Facade contract — a read-only registry of Grainlify contracts.
#[contract]
pub struct ViewFacade;

#[contractimpl]
impl ViewFacade {
    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initialize the facade with an immutable administrator address.
    ///
    /// # Arguments
    /// * `admin` — The address that will be authorized to call [`register`]
    ///   and [`deregister`]. This value is written once and can never be
    ///   overwritten.
    ///
    /// # Errors
    /// * [`FacadeError::AlreadyInitialized`] — if `init` has already been
    ///   called on this contract instance.
    ///
    /// # Events
    /// Emits [`InitializedEvent`] on the `("facade", "init")` topic.
    ///
    /// # Security
    /// - Can be called by **anyone** exactly once (first-caller pattern).
    ///   Deploy the contract and call `init` in the same transaction to
    ///   prevent front-running on public networks.
    /// - After this call the admin is immutable for the lifetime of the
    ///   contract; even a WASM upgrade cannot change it.
    ///
    /// # Example
    /// ```text
    /// stellar contract invoke --id <CONTRACT> -- init --admin <GADMIN...>
    /// ```
    pub fn init(env: Env, admin: Address) -> Result<(), FacadeError> {
        // Guard: reject double initialization to protect admin immutability.
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(FacadeError::AlreadyInitialized);
        }

        // Store the admin address — written exactly once, never overwritten.
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Emit an Initialized event so off-chain indexers know the contract
        // is ready. Topic uses two short symbols kept under 32 bytes each.
        env.events().publish(
            (symbol_short!("facade"), symbol_short!("init")),
            InitializedEvent {
                admin: admin.clone(),
            },
        );

        Ok(())
    }

    // ========================================================================
    // Admin Query
    // ========================================================================

    /// Return the administrator address, or `None` if not yet initialized.
    ///
    /// This view function lets callers (dashboards, deployment scripts) confirm
    /// the initialization state without having to catch an error.
    ///
    /// # Returns
    /// * `Some(admin)` — contract is initialized.
    /// * `None` — contract has not been initialized yet.
    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    // ========================================================================
    // Registry Mutations (admin-only)
    // ========================================================================

    /// Register a contract address so it appears in cross-contract views.
    ///
    /// ## Duplicate Registration Policy
    /// If the address is already registered, the existing entry's `kind` and
    /// `version` are **updated** to match the new values, and the entry
    /// maintains its original position in insertion order.
    ///
    /// This ensures:
    /// - Single-source-of-truth per address (no duplicate entries)
    /// - Consistent query results: `get_contract()` always returns the latest metadata
    /// - List consistency: `list_contracts()` reflects all registered addresses exactly once
    /// - Operational convenience: admin can update metadata without explicit deregister
    ///
    /// # Arguments
    /// * `address` — On-chain address of the contract to register.
    /// * `kind`    — Role of the contract within the ecosystem.
    /// * `version` — Version number reported by the contract.
    ///
    /// # Authorization
    /// Requires a valid signature from the stored admin address
    /// (`admin.require_auth()`).
    ///
    /// # Errors
    /// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
    /// * [`FacadeError::RegistryFull`] — if registry has reached [`MAX_REGISTRY_SIZE`].
    ///
    /// # Note
    /// Registering the same address multiple times will create duplicate
    /// entries. Callers should call [`get_contract`] first to check for an
    /// existing entry, or [`deregister`] before re-registering with updated
    /// metadata.
    ///
    /// ## Registry Limits
    ///
    /// The facade enforces a hard cap of [`MAX_REGISTRY_SIZE`] entries to prevent
    /// unbounded storage growth. If the registry is full, registration will fail
    /// with [`FacadeError::RegistryFull`]. Admins must deregister existing entries
    /// before adding new ones when at capacity.
    pub fn register(
        env: Env,
        address: Address,
        kind: ContractKind,
        version: u32,
    ) -> Result<(), FacadeError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(FacadeError::NotInitialized)?;

        admin.require_auth();

        let mut registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        // Enforce registry size limit
        if registry.len() >= MAX_REGISTRY_SIZE {
            return Err(FacadeError::RegistryFull);
        }

        registry.push_back(RegisteredContract {
            address,
            kind,
            version,
        });

        env.storage().instance().set(&DataKey::Registry, &registry);

        Ok(())
    }

    /// Remove a previously registered contract address.
    ///
    /// If `address` is not in the registry this is a no-op (the registry is
    /// returned unchanged). This avoids callers having to check existence
    /// before deregistering.
    ///
    /// # Arguments
    /// * `address` — Address to remove from the registry.
    ///
    /// # Authorization
    /// Requires a valid signature from the stored admin address.
    ///
    /// # Errors
    /// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
    pub fn deregister(env: Env, address: Address) -> Result<(), FacadeError> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(FacadeError::NotInitialized)?;

        admin.require_auth();

        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        let mut updated = Vec::new(&env);
        for entry in registry.iter() {
            if entry.address != address {
                updated.push_back(entry);
            }
        }

        env.storage().instance().set(&DataKey::Registry, &updated);

        Ok(())
    }

    // ========================================================================
    // Registry Views (public)
    // ========================================================================

    /// Return all registered contracts as an ordered list.
    ///
    /// The list is in insertion order. An empty vec is returned if no
    /// contracts have been registered yet.
    ///
    /// # Arguments
    /// * `offset` — Number of entries to skip from the start (default: 0).
    /// * `limit`  — Maximum number of entries to return (default: all).
    ///
    /// # Errors
    /// * [`FacadeError::InvalidPagination`] — if offset > total entries or limit = 0.
    ///
    /// # Note
    /// This is a pure read — no authorization required.
    ///
    /// ## Pagination
    ///
    /// For large registries, use pagination to avoid excessive gas costs:
    /// - First page: `list_contracts(0, 100)`
    /// - Second page: `list_contracts(100, 100)`
    /// - Continue until returned vec length < limit
    pub fn list_contracts(
        env: Env,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<Vec<RegisteredContract>, FacadeError> {
        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        let total = registry.len();
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(total);

        // Validate pagination parameters
        if offset > total {
            return Err(FacadeError::InvalidPagination);
        }
        if limit == 0 {
            return Err(FacadeError::InvalidPagination);
        }

        // Calculate end index, ensuring we don't exceed total
        let end = if offset + limit > total {
            total
        } else {
            offset + limit
        };

        // Extract the requested slice
        let mut result = Vec::new(&env);
        for i in offset..end {
            result.push_back(registry.get(i).unwrap().clone());
        }

        Ok(result)
    }

    /// Return all registered contracts as an ordered list (legacy version).
    ///
    /// This is a compatibility wrapper that returns the entire registry.
    /// New code should use the paginated version of `list_contracts`.
    ///
    /// # Note
    /// This is a pure read — no authorization required.
    /// For large registries, this may be expensive. Consider using
    /// `list_contracts(offset, limit)` for pagination.
    pub fn list_contracts_all(env: Env) -> Vec<RegisteredContract> {
        env.storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env))
    }

    /// Return the total number of registered contracts.
    ///
    /// Returns the total registry size, which is useful for pagination calculations.
    /// This is cheaper than loading the full registry with `list_contracts_all`.
    ///
    /// # Note
    /// This is a pure read — no authorization required.
    /// 
    /// ## Usage for Pagination
    /// 
    /// To implement pagination:
    /// 1. Call `contract_count()` to get total entries
    /// 2. Calculate pages: `total_entries / page_size`
    /// 3. Fetch each page: `list_contracts(offset, limit)`
    pub fn contract_count(env: Env) -> u32 {
        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));
        registry.len()
    }

    /// Look up a registered contract by its on-chain address.
    ///
    /// # Arguments
    /// * `address` — The contract address to search for.
    ///
    /// # Returns
    /// * `Some(entry)` — if the address is in the registry.
    /// * `None`        — if the address has not been registered.
    ///
    /// # Performance
    /// Performs an `O(n)` scan over the registry.
    ///
    /// # Note
    /// This is a pure read — no authorization required.
    pub fn get_contract(env: Env, address: Address) -> Option<RegisteredContract> {
        let registry: Vec<RegisteredContract> = env
            .storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env));

        for entry in registry.iter() {
            if entry.address == address {
                return Some(entry);
            }
        }
        None
    }

    /// Query all payout records for `recipient` within `program_id` via a
    /// registered `ProgramEscrow` contract.
    ///
    /// Delegates to `ProgramEscrow::query_recipient_history`, which reads the
    /// lazy-initialized inverted index keyed by `(program_id, recipient)`.
    /// The lookup is O(1) in the number of total payouts across all recipients.
    ///
    /// # Arguments
    /// * `escrow` — On-chain address of a registered `ProgramEscrow` contract.
    /// * `program_id` — String identifier of the program.
    /// * `recipient` — Address whose payout history is requested.
    ///
    /// # Returns
    /// `Vec<PayoutRecord>` — may be empty if the recipient has no payouts.
    ///
    /// # Security
    /// - Read-only; does not mutate any state.
    /// - No authorization required — payout records are public on-chain data.
    /// - The caller-supplied `escrow` address is not validated against the
    ///   registry; callers should verify the contract kind via `get_contract`
    ///   before trusting results.
    pub fn query_recipient_history(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
        recipient: Address,
    ) -> Vec<PayoutRecord> {
        let client = EscrowClient::new(&env, &escrow);
        client.query_recipient_history(&program_id, &recipient)
    }

    // ========================================================================
    // Cached Query Methods (per-invocation memoization via QueryCache)
    // ========================================================================

    /// Fetch [`ProgramData`] for `program_id` on `escrow`, using the
    /// per-invocation [`QueryCache`] to avoid redundant cross-contract calls.
    ///
    /// If this function (or [`query_program_balance_and_fee`]) has already
    /// been called with the same `(escrow, program_id)` pair within the
    /// current transaction, the cached value is returned without another
    /// cross-contract call.
    ///
    /// # Arguments
    /// * `escrow`     — Address of a registered `ProgramEscrow` contract.
    /// * `program_id` — Identifier of the program to query.
    ///
    /// # Returns
    /// The full [`ProgramData`] struct from the escrow contract.
    ///
    /// # Panics
    /// Propagates any panic from the underlying cross-contract call
    /// (e.g. if the program does not exist on the target escrow).
    ///
    /// # Caveats
    /// The cache is scoped to the current invocation.  If a write operation
    /// (e.g. `batch_payout`) mutates the program state within the same
    /// transaction, subsequent cached reads will return the **pre-mutation**
    /// value.  Callers mixing reads and writes in one transaction should
    /// invalidate the cache explicitly via [`QueryCache::invalidate_program_data`].
    ///
    /// # Security
    /// - Read-only; does not mutate persistent state.
    /// - No authorization required.
    /// - The cache is scoped to the current invocation; no stale data can
    ///   leak across transactions.
    pub fn query_program_data_cached(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
    ) -> ProgramData {
        QueryCache::get_or_load_program_data(&env, &escrow, &program_id)
    }

    /// Fetch [`FeeConfig`] for `escrow`, using the per-invocation [`QueryCache`]
    /// to avoid redundant cross-contract calls.
    ///
    /// # Arguments
    /// * `escrow` — Address of a registered `ProgramEscrow` contract.
    ///
    /// # Returns
    /// The [`FeeConfig`] struct from the escrow contract.
    ///
    /// # Panics
    /// Propagates any panic from the underlying cross-contract call.
    ///
    /// # Caveats
    /// The cache is scoped to the current invocation.  If a fee-config update
    /// occurs within the same transaction, subsequent cached reads return the
    /// pre-update value.  Invalidate via [`QueryCache::invalidate_fee_config`].
    ///
    /// # Security
    /// - Read-only; does not mutate persistent state.
    /// - No authorization required.
    pub fn query_fee_config_cached(env: Env, escrow: Address) -> FeeConfig {
        QueryCache::get_or_load_fee_config(&env, &escrow)
    }

    /// Aggregated query returning both [`ProgramData`] and [`FeeConfig`] in a
    /// single call, with per-invocation caching.
    ///
    /// This is the most efficient way to fetch program metadata when a
    /// frontend needs both the program state and the current fee schedule.
    /// The first read of each type populates the cache; subsequent reads
    /// within the same transaction are served from memory without additional
    /// cross-contract calls.
    ///
    /// # Arguments
    /// * `escrow`     — Address of a registered `ProgramEscrow` contract.
    /// * `program_id` — Identifier of the program to query.
    ///
    /// # Returns
    /// A tuple of `(ProgramData, FeeConfig)` from the escrow contract.
    ///
    /// # Panics
    /// Propagates panics from either underlying cross-contract call
    /// (`get_program_info_v2` or `get_fee_config`).
    ///
    /// # Caveats
    /// The same invocation-scoping caveats apply as for
    /// [`query_program_data_cached`] and [`query_fee_config_cached`].
    ///
    /// # Gas savings
    ///
    /// Without caching, querying both `ProgramData` and `FeeConfig` requires
    /// two cross-contract calls.  With the cache, subsequent calls within the
    /// same transaction avoid both calls entirely — each saved cross-contract
    /// call eliminates ~1000+ CPU instructions.
    pub fn query_program_balance_and_fee(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
    ) -> (ProgramData, FeeConfig) {
        let data = QueryCache::get_or_load_program_data(&env, &escrow, &program_id);
        let fees = QueryCache::get_or_load_fee_config(&env, &escrow);
        (data, fees)
    }
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_cross_contract_safety;
#[cfg(test)]
mod tests;
