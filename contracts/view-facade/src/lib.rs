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
//! ## Query Notes
//!
//! - `list_contracts` returns entries in registration order.
//! - `contract_count` mirrors the current registry length.
//! - `get_contract` performs an `O(n)` scan and returns the first matching
//!   entry for the requested address.
//! - `O(n)` scans are acceptable for the intended small registry size, but
//!   callers should avoid treating this facade as an unbounded index.
//!
//! ## Query Flow
//!
//! 1. Call `contract_count` to size the expected dashboard result.
//! 2. Call `list_contracts` to render the full registry in registration order.
//! 3. Call `get_contract` when the UI needs to refresh a single known address.
//!
//! ## Security Model
//!
//! - **No fund custody**: this contract holds no tokens and transfers no funds.
//! - **No external writes**: it writes state only to its own instance storage.
//! - **Immutable admin**: the administrator address is set once at initialization and
//!   can never be changed, preventing privilege escalation after deployment.
//! - **Double-init protection**: a second call to [`ViewFacade::init`] is rejected
//!   with [`FacadeError::AlreadyInitialized`], so the initial admin cannot be replaced.
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
    /// `init` was called on a contract that has already been initialized.
    ///
    /// The admin address is immutable after the first successful `init`.
    /// Re-initialization is explicitly rejected to prevent privilege escalation.
    AlreadyInitialized = 1,

    /// An admin-gated entry-point was called before `init` was invoked.
    ///
    /// Deploy then call `init(admin)` before registering any contracts.
    NotInitialized = 2,
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
    ///
    /// # Note
    /// Registering the same address multiple times will create duplicate
    /// entries. Callers should call [`get_contract`] first to check for an
    /// existing entry, or [`deregister`] before re-registering with updated
    /// metadata.
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
    /// # Note
    /// This is a pure read — no authorization required.
    pub fn list_contracts(env: Env) -> Vec<RegisteredContract> {
        env.storage()
            .instance()
            .get(&DataKey::Registry)
            .unwrap_or(Vec::new(&env))
    }

    /// Return the total number of registered contracts.
    ///
    /// Equivalent to `list_contracts().len()` but cheaper because it avoids
    /// deserializing the full entry list.
    ///
    /// # Note
    /// This is a pure read — no authorization required.
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
