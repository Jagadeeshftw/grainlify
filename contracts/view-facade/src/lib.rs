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
//! ## Module layout
//!
//! This crate is split into a **small entrypoint module** ([`ViewFacade`]) plus the
//! query-specific adapters that keep the entrypoint surface easy to audit:
//!
//! - [`types`] — the registry data structures, error type and storage keys.
//! - [`registry`] — the registry read/write helpers used by the entrypoint.
//! - [`query`] — the cross-contract clients, per-invocation [`query::QueryCache`] and
//!   mirrored program-escrow types.
//!
//! The public contract ABI (entrypoint names, argument order and return types) is
//! unchanged by this split.
//!
//! ## Query Notes
//!
//! - `list_contracts` supports pagination with optional `offset` and `limit` parameters.
//! - `list_contracts_all` returns the full registry (legacy compatibility).
//! - `contract_count` returns the total registry size for pagination calculations.
//! - `get_contract` performs an `O(n)` scan and returns the first matching
//!   entry for the requested address.
//! - Registry size is bounded by [`types::MAX_REGISTRY_SIZE`] (1000 entries) to prevent
//!   unbounded storage growth.
//!
//! ## Security Model
//!
//! - **No fund custody**: this contract holds no tokens and transfers no funds.
//! - **No external writes**: it writes state only to its own instance storage.
//! - **Immutable admin**: the administrator address is set once at initialization and
//!   can never be changed, preventing privilege escalation after deployment.
//! - **Double-init protection**: a second call to [`ViewFacade::init`] is rejected
//!   with [`types::FacadeError::AlreadyInitialized`], so the initial admin cannot be replaced.
//! - **Bounded registry**: hard cap on entries prevents storage bloat attacks.
//!
//! ## ABI Stability
//!
//! The complete public interface of this contract — including stability classifications
//! (`STABLE` / `EVOLVING` / `INTERNAL`), breaking-change rules, and all types that are
//! duplicated in facade bindings — is documented in the cross-contract ABI stability matrix:
//!
//! **[`docs/abi-stability-matrix.md`](../../../../docs/abi-stability-matrix.md)**
//!
//! ### Synchronization risks in this crate
//! - The local `PayoutRecord` struct is a **subset** of the canonical `PayoutRecord` in
//!   `program-escrow/src/lib.rs`: it omits the `payout_type` field. Any field addition or
//!   reorder in the canonical struct must be reflected here in the same PR, or XDR decoding
//!   of `query_recipient_history` responses will silently produce incorrect values.
//! - `ContractKind` variants must remain in sync with `grainlify-core`'s registry entries.

mod registry;
mod types;

pub mod query;

pub use query::{EscrowClient, EscrowQueryClient, PayoutRecord, QueryCache, QueryCacheKey};
pub use registry::{
    contract_count, deregister, get_admin, get_contract, init, list_contracts,
    list_contracts_all, register,
};
pub use types::{
    ContractKind, DataKey, FacadeError, InitializedEvent, RegisteredContract, MAX_REGISTRY_SIZE,
};

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

/// The View Facade contract — a read-only registry of Grainlify contracts.
///
/// All entrypoints are thin delegates to the query/registry adapters so that the
/// full read-only ABI surface is visible in a single small module.
#[contract]
pub struct ViewFacade;

#[contractimpl]
impl ViewFacade {
    /// Initialize the facade with an immutable administrator address.
    ///
    /// # Errors
    /// * [`FacadeError::AlreadyInitialized`] — if `init` has already been called.
    ///
    /// # Security
    /// - Can be called by **anyone** exactly once (first-caller pattern).
    /// - After this call the admin is immutable for the lifetime of the contract.
    pub fn init(env: Env, admin: Address) -> Result<(), FacadeError> {
        init(&env, &admin)
    }

    /// Return the administrator address, or `None` if not yet initialized.
    pub fn get_admin(env: Env) -> Option<Address> {
        get_admin(&env)
    }

    /// Register a contract address so it appears in cross-contract views.
    ///
    /// # Authorization
    /// Requires a valid signature from the stored admin address.
    ///
    /// # Errors
    /// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
    /// * [`FacadeError::RegistryFull`] — if registry has reached [`MAX_REGISTRY_SIZE`].
    pub fn register(
        env: Env,
        address: Address,
        kind: ContractKind,
        version: u32,
    ) -> Result<(), FacadeError> {
        register(&env, &address, &kind, version)
    }

    /// Remove a previously registered contract address.
    ///
    /// # Authorization
    /// Requires a valid signature from the stored admin address.
    ///
    /// # Errors
    /// * [`FacadeError::NotInitialized`] — if `init` has not yet been called.
    pub fn deregister(env: Env, address: Address) -> Result<(), FacadeError> {
        deregister(&env, &address)
    }

    /// Return all registered contracts as an ordered list.
    ///
    /// # Arguments
    /// * `offset` — Number of entries to skip from the start (default: 0).
    /// * `limit`  — Maximum number of entries to return (default: all).
    ///
    /// # Errors
    /// * [`FacadeError::InvalidPagination`] — if offset > total entries or limit = 0.
    pub fn list_contracts(
        env: Env,
        offset: Option<u32>,
        limit: Option<u32>,
    ) -> Result<Vec<RegisteredContract>, FacadeError> {
        list_contracts(&env, offset, limit)
    }

    /// Return all registered contracts as an ordered list (legacy version).
    pub fn list_contracts_all(env: Env) -> Vec<RegisteredContract> {
        list_contracts_all(&env)
    }

    /// Return the total number of registered contracts.
    pub fn contract_count(env: Env) -> u32 {
        contract_count(&env)
    }

    /// Look up a registered contract by its on-chain address.
    pub fn get_contract(env: Env, address: Address) -> Option<RegisteredContract> {
        get_contract(&env, &address)
    }

    /// Query all payout records for `recipient` within `program_id` via a
    /// registered `ProgramEscrow` contract.
    pub fn query_recipient_history(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
        recipient: Address,
    ) -> Vec<PayoutRecord> {
        query::recipient_history(&env, &escrow, &program_id, &recipient)
    }

    /// Fetch [`query::ProgramData`] for `program_id` on `escrow`, using the
    /// per-invocation [`query::QueryCache`] to avoid redundant cross-contract calls.
    pub fn query_program_data_cached(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
    ) -> query::ProgramData {
        query::program_data_cached(&env, &escrow, &program_id)
    }

    /// Fetch [`query::FeeConfig`] for `escrow`, using the per-invocation
    /// [`query::QueryCache`] to avoid redundant cross-contract calls.
    pub fn query_fee_config_cached(env: Env, escrow: Address) -> query::FeeConfig {
        query::fee_config_cached(&env, &escrow)
    }

    /// Aggregated query returning both [`query::ProgramData`] and [`query::FeeConfig`]
    /// in a single call, with per-invocation caching.
    pub fn query_program_balance_and_fee(
        env: Env,
        escrow: Address,
        program_id: soroban_sdk::String,
    ) -> (query::ProgramData, query::FeeConfig) {
        query::program_balance_and_fee(&env, &escrow, &program_id)
    }
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_cross_contract_safety;
#[cfg(test)]
mod tests;
