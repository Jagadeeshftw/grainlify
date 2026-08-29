//! Query-specific adapters for the view-facade contract.
//!
//! This module owns everything that touches contracts *other* than the facade
//! itself: the cross-contract client traits, the mirrored program-escrow types,
//! and the per-invocation [`QueryCache`]. The contract entrypoint in
//! [`crate::ViewFacade`] delegates to the functions in this module so that the
//! read-only query behavior is contained in one auditable place.

pub use program_escrow::{FeeConfig, ProgramData};
use soroban_sdk::{contracttype, Address, Env, String, Vec};

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
        program_id: String,
        recipient: Address,
    ) -> Vec<PayoutRecord>;
}

/// Cross-contract client for querying ProgramData and FeeConfig from a
/// ProgramEscrow contract. Used by [`QueryCache`] to fetch data that is
/// then memoized in temporary storage.
#[soroban_sdk::contractclient(name = "EscrowQueryClient")]
pub trait ProgramEscrowQueryTrait {
    /// Fetch the full [`ProgramData`] struct for `program_id`.
    fn get_program_info_v2(env: Env, program_id: String) -> ProgramData;

    /// Fetch the current [`FeeConfig`] from the escrow contract.
    fn get_fee_config(env: Env) -> FeeConfig;
}

/// Storage keys used by [`QueryCache`] in Soroban temporary storage.
///
/// Temporary storage is scoped to the current top-level contract invocation
/// and is automatically discarded at the end of the transaction.
#[contracttype]
pub enum QueryCacheKey {
    /// Cached [`ProgramData`] for a specific `(escrow_address, program_id)` pair.
    ProgramData(Address, String),
    /// Cached [`FeeConfig`] for a specific escrow contract address.
    FeeConfig(Address),
}

/// Per-invocation read-through cache backed by Soroban temporary storage.
///
/// # Purpose
///
/// When multiple query functions within a single transaction call into the same
/// escrow contract to fetch [`ProgramData`] or [`FeeConfig`], each call incurs a
/// separate storage-read cost. `QueryCache` memoizes the results in temporary
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
pub struct QueryCache;

impl QueryCache {
    /// Return [`ProgramData`] for `program_id` on `escrow`, fetching from the
    /// underlying contract only on first access within the current invocation.
    ///
    /// # Panics
    /// If the cross-contract call to `get_program_info_v2` panics (e.g.
    /// program not found), the panic propagates to the caller.
    pub fn get_or_load_program_data(
        env: &Env,
        escrow: &Address,
        program_id: &String,
    ) -> ProgramData {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());

        if let Some(cached) = env.storage().temporary().get::<QueryCacheKey, ProgramData>(&key) {
            return cached;
        }

        let client = EscrowQueryClient::new(env, escrow);
        let data = client.get_program_info_v2(program_id);

        env.storage().temporary().set(&key, &data);

        data
    }

    /// Return [`FeeConfig`] for `escrow`, fetching from the underlying contract
    /// only on first access within the current invocation.
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
    pub fn invalidate_program_data(env: &Env, escrow: &Address, program_id: &String) {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());
        env.storage().temporary().remove(&key);
    }

    /// Explicitly remove a cached [`FeeConfig`] entry from temporary storage.
    pub fn invalidate_fee_config(env: &Env, escrow: &Address) {
        let key = QueryCacheKey::FeeConfig(escrow.clone());
        env.storage().temporary().remove(&key);
    }
}

/// Query all payout records for `recipient` within `program_id` via a
/// registered `ProgramEscrow` contract.
///
/// # Returns
/// `Vec<PayoutRecord>` — may be empty if the recipient has no payouts.
pub fn recipient_history(
    env: &Env,
    escrow: &Address,
    program_id: &String,
    recipient: &Address,
) -> Vec<PayoutRecord> {
    let client = EscrowClient::new(env, escrow);
    client.query_recipient_history(program_id, recipient)
}

/// Fetch [`ProgramData`] for `program_id` on `escrow`, using the
/// per-invocation [`QueryCache`] to avoid redundant cross-contract calls.
pub fn program_data_cached(env: &Env, escrow: &Address, program_id: &String) -> ProgramData {
    QueryCache::get_or_load_program_data(env, escrow, program_id)
}

/// Fetch [`FeeConfig`] for `escrow`, using the per-invocation [`QueryCache`].
pub fn fee_config_cached(env: &Env, escrow: &Address) -> FeeConfig {
    QueryCache::get_or_load_fee_config(env, escrow)
}

/// Aggregated query returning both [`ProgramData`] and [`FeeConfig`] in a
/// single call, with per-invocation caching.
pub fn program_balance_and_fee(
    env: &Env,
    escrow: &Address,
    program_id: &String,
) -> (ProgramData, FeeConfig) {
    let data = QueryCache::get_or_load_program_data(env, escrow, program_id);
    let fees = QueryCache::get_or_load_fee_config(env, escrow);
    (data, fees)
}
