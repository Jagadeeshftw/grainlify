#![no_std]
//! # Escrow View Facade
//!
//! Read-only aggregation of bounty-escrow data for frontend consumption; also proxies
//! delegate queries to program-escrow. Returns `None` / empty vecs instead of trapping
//! when underlying contracts return errors, making it safe to call from UI code.
//!
//! ## Module layout
//!
//! This crate is split into a **small entrypoint module** ([`EscrowViewFacade`]) plus
//! the query-specific adapters:
//!
//! - [`types`] — the public `#[contracttype]` data structures.
//! - [`query`] — the cross-contract bindings, per-invocation [`query::QueryCache`] and
//!   the "safe" read helpers that map cross-contract errors to `None` / empty results.
//!
//! The public contract ABI (entrypoint names, argument order and return types) is
//! unchanged by this split.
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
//! - `EscrowStatus` (local re-declaration in `types.rs`) must stay in sync with
//!   `bounty-escrow`'s canonical enum. Variant reorder or removal is XDR-breaking.
//! - `query::bounty_escrow` mirrors `EscrowStatus`, `EscrowMetadata`, `PauseFlags`,
//!   `Escrow`, `EscrowWithId`, and `AnonymousParty` from `bounty-escrow`. All must be
//!   updated in the same PR as any change to their canonical counterparts.
//! - `query::program_escrow` mirrors `ProgramDelegateInfo` from `program-escrow`.
//!   Field additions or reorders must be applied to the binding simultaneously.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

use program_escrow::{FeeConfig, ProgramData};

mod bounty_escrow {
    include!("bounty_escrow_bindings.rs");
}

/// Cross-contract client for ProgramEscrow data queries used by the cache.
#[soroban_sdk::contractclient(name = "EscrowDataClient")]
pub trait ProgramEscrowDataQueryTrait {
    fn get_program_info_v2(env: Env, program_id: String) -> ProgramData;
    fn get_fee_config(env: Env) -> FeeConfig;
}

/// Storage keys for the [`QueryCache`] in Soroban temporary storage.
///
/// # Note: Keep in sync with `view-facade/src/lib.rs` `QueryCacheKey`.
#[contracttype]
pub enum QueryCacheKey {
    ProgramData(Address, String),
    FeeConfig(Address),
}

/// Per-invocation read-through cache for ProgramEscrow queries.
///
/// When the facade aggregates data from multiple escrow contracts or makes
/// repeated reads of the same contract within a single transaction, this
/// cache eliminates redundant cross-contract calls.
///
/// # Safety
/// - Read-only: never mutates persistent storage.
/// - Temporary storage is automatically discarded at transaction end.
pub struct QueryCache;

impl QueryCache {
    /// Get [`ProgramData`] for `program_id` on `escrow`, caching the result.
    pub fn get_or_load_program_data(
        env: &Env,
        escrow: &Address,
        program_id: &String,
    ) -> ProgramData {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());
        if let Some(cached) = env
            .storage()
            .temporary()
            .get::<QueryCacheKey, ProgramData>(&key)
        {
            return cached;
        }
        let client = EscrowDataClient::new(env, escrow);
        let data = client.get_program_info_v2(program_id);
        env.storage().temporary().set(&key, &data);
        data
    }

    /// Get [`FeeConfig`] for `escrow`, caching the result.
    pub fn get_or_load_fee_config(env: &Env, escrow: &Address) -> FeeConfig {
        let key = QueryCacheKey::FeeConfig(escrow.clone());
        if let Some(cached) = env
            .storage()
            .temporary()
            .get::<QueryCacheKey, FeeConfig>(&key)
        {
            return cached;
        }
        let client = EscrowDataClient::new(env, escrow);
        let config = client.get_fee_config();
        env.storage().temporary().set(&key, &config);
        config
    }

pub use query::{EscrowDataClient, QueryCache, QueryCacheKey};
pub use types::{EscrowStatus, EscrowSummary, UserPortfolio};

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

/// The Escrow View Facade — a read-only aggregation layer for escrow data.
#[contract]
pub struct EscrowViewFacade;

#[contractimpl]
impl EscrowViewFacade {
    /// Safely retrieve an aggregated summary of a single escrow.
    /// Returns `None` if the escrow does not exist instead of trapping.
    pub fn get_escrow_summary(
        env: Env,
        escrow_contract: Address,
        bounty_id: u64,
    ) -> Option<EscrowSummary> {
        query::get_escrow_summary(&env, &escrow_contract, bounty_id)
    }

    /// Retrieve summaries for a batch of `bounty_ids`.
    /// Missing escrows are omitted from the result vector.
    pub fn get_escrow_summaries(
        env: Env,
        escrow_contract: Address,
        bounty_ids: Vec<u64>,
    ) -> Vec<EscrowSummary> {
        query::get_escrow_summaries(&env, &escrow_contract, bounty_ids)
    }

    /// Retrieve an aggregated view of a user's portfolio, including both the
    /// escrows they deposited into and escrows they are listed to receive.
    pub fn get_user_portfolio(env: Env, escrow_contract: Address, user: Address) -> UserPortfolio {
        query::get_user_portfolio(&env, &escrow_contract, &user)
    }

    /// Query all current delegate assignments for a program escrow.
    ///
    /// Returns an empty vector if the query fails, preserving the facade's
    /// read-only auditing semantics.
    pub fn query_all_delegates(
        env: Env,
        program_contract: Address,
        program_id: String,
    ) -> Vec<program_escrow::ProgramDelegateInfo> {
        let client = program_escrow::ProgramEscrowContractClient::new(&env, &program_contract);
        let delegates_res = client.try_query_all_delegates(&program_id);

        if let Ok(Ok(delegates)) = delegates_res {
            delegates
        } else {
            Vec::new(&env)
        }
    }

    /// Query the payout history for a specific recipient within a program.
    ///
    /// Returns an empty `Vec` if the recipient has never received a payout.
    pub fn query_recipient_history(
        env: Env,
        program_contract: Address,
        program_id: String,
        recipient: Address,
    ) -> Vec<program_escrow::PayoutRecord> {
        let client = program_escrow::ProgramEscrowContractClient::new(&env, &program_contract);
        let result = client.try_query_recipient_history(&program_id, &recipient);

        if let Ok(Ok(records)) = result {
            records
        } else {
            Vec::new(&env)
        }
    }

    /// Fetch [`query::ProgramData`] for `program_id` on `escrow`, using the
    /// per-invocation [`QueryCache`] to avoid redundant cross-contract calls.
    pub fn query_program_data_cached(
        env: Env,
        escrow: Address,
        program_id: String,
    ) -> query::ProgramData {
        query::query_program_data_cached(&env, &escrow, &program_id)
    }

    /// Fetch [`query::FeeConfig`] for `escrow`, using the per-invocation [`QueryCache`].
    pub fn query_fee_config_cached(env: Env, escrow: Address) -> query::FeeConfig {
        query::query_fee_config_cached(&env, &escrow)
    }

    /// Aggregated query returning both [`query::ProgramData`] and [`query::FeeConfig`]
    /// in a single call, with per-invocation caching for efficiency.
    pub fn query_program_balance_and_fee(
        env: Env,
        escrow: Address,
        program_id: String,
    ) -> (query::ProgramData, query::FeeConfig) {
        query::query_program_balance_and_fee(&env, &escrow, &program_id)
    }
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_cross_contract_safety;
#[cfg(test)]
mod test_query_adapters;
