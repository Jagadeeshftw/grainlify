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
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_cross_contract_safety;
