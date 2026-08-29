//! Query-specific adapters for the escrow-view-facade contract.
//!
//! This module owns everything that talks to contracts *other* than the facade
//! itself: the minimal bindings for `BountyEscrow` and `ProgramEscrow`, the
//! per-invocation [`QueryCache`], and the "safe" read helpers that translate
//! cross-contract errors into `None` / empty results.
//!
//! The contract entrypoint in [`crate::EscrowViewFacade`] delegates to the
//! functions in this module.

use soroban_sdk::{contracttype, Address, Env, String, Vec};

pub use ::program_escrow::{FeeConfig, ProgramData};

use crate::types::{EscrowStatus, EscrowSummary, UserPortfolio};

pub mod bounty_escrow {
    include!("bounty_escrow_bindings.rs");
}

pub mod program_escrow {
    include!("program_escrow_bindings.rs");
}

/// Cross-contract client for ProgramEscrow data queries used by the cache.
#[soroban_sdk::contractclient(name = "EscrowDataClient")]
pub trait ProgramEscrowDataQueryTrait {
    fn get_program_info_v2(env: Env, program_id: String) -> ProgramData;
    fn get_fee_config(env: Env) -> FeeConfig;
}

/// Storage keys for the [`QueryCache`] in Soroban temporary storage.
///
/// # Note: Keep in sync with `view-facade/src/query.rs` `QueryCacheKey`.
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

    /// Remove a cached [`ProgramData`] entry (for testing).
    pub fn invalidate_program_data(env: &Env, escrow: &Address, program_id: &String) {
        let key = QueryCacheKey::ProgramData(escrow.clone(), program_id.clone());
        env.storage().temporary().remove(&key);
    }

    /// Remove a cached [`FeeConfig`] entry (for testing).
    pub fn invalidate_fee_config(env: &Env, escrow: &Address) {
        let key = QueryCacheKey::FeeConfig(escrow.clone());
        env.storage().temporary().remove(&key);
    }
}

/// Map a BountyEscrow `EscrowStatus` into the facade's public [`EscrowStatus`].
fn map_status(status: &bounty_escrow::EscrowStatus) -> EscrowStatus {
    match status {
        bounty_escrow::EscrowStatus::Locked => EscrowStatus::Locked,
        bounty_escrow::EscrowStatus::Released => EscrowStatus::Released,
        bounty_escrow::EscrowStatus::Refunded => EscrowStatus::Refunded,
        bounty_escrow::EscrowStatus::PartiallyRefunded => EscrowStatus::PartiallyRefunded,
    }
}

/// Fetch the pause flags from a BountyEscrow contract, mapping errors to
/// "not paused" so read-only calls never trap.
fn is_escrow_contract_paused(env: &Env, client: &bounty_escrow::Client) -> bool {
    let pause_flags_res = client.try_get_pause_flags();
    if let Ok(Ok(flags)) = pause_flags_res {
        flags.lock_paused || flags.release_paused || flags.refund_paused
    } else {
        false
    }
}

/// Build an [`EscrowSummary`] for a single `bounty_id`, returning `None` when
/// the escrow (or its metadata) is missing so callers never trap.
fn build_summary(
    env: &Env,
    client: &bounty_escrow::Client,
    id: u64,
    info: &bounty_escrow::Escrow,
    is_paused: bool,
) -> EscrowSummary {
    let metadata_res = client.try_get_metadata(&id);
    let (repo_id, issue_id, bounty_type) = if let Ok(Ok(meta)) = metadata_res {
        (meta.repo_id, meta.issue_id, meta.bounty_type)
    } else {
        (0, 0, String::from_str(env, ""))
    };

    EscrowSummary {
        bounty_id: id,
        depositor: info.depositor.clone(),
        amount: info.amount,
        remaining_amount: info.remaining_amount,
        status: map_status(&info.status),
        deadline: info.deadline,
        repo_id,
        issue_id,
        bounty_type,
        is_paused,
    }
}

/// Safely retrieve an aggregated summary of a single escrow.
///
/// Returns `None` if the escrow does not exist instead of trapping.
pub fn get_escrow_summary(
    env: &Env,
    escrow_contract: &Address,
    bounty_id: u64,
) -> Option<EscrowSummary> {
    let client = bounty_escrow::Client::new(env, escrow_contract);

    let escrow_info_res = client.try_get_escrow_info(&bounty_id);

    if let Ok(Ok(info)) = escrow_info_res {
        let is_paused = is_escrow_contract_paused(env, &client);
        Some(build_summary(env, &client, bounty_id, &info, is_paused))
    } else {
        None
    }
}

/// Retrieve summaries for a batch of `bounty_ids`.
///
/// Missing escrows are omitted from the result vector.
pub fn get_escrow_summaries(
    env: &Env,
    escrow_contract: &Address,
    bounty_ids: Vec<u64>,
) -> Vec<EscrowSummary> {
    let mut summaries = Vec::new(env);

    let client = bounty_escrow::Client::new(env, escrow_contract);
    let is_paused = is_escrow_contract_paused(env, &client);

    for id in bounty_ids.iter() {
        let escrow_info_res = client.try_get_escrow_info(&id);
        if let Ok(Ok(info)) = escrow_info_res {
            summaries.push_back(build_summary(env, &client, id, &info, is_paused));
        }
    }
    summaries
}

/// Retrieve an aggregated view of a user's portfolio, including both the
/// escrows they deposited into and escrows they are listed to receive.
pub fn get_user_portfolio(env: &Env, escrow_contract: &Address, user: &Address) -> UserPortfolio {
    let client = bounty_escrow::Client::new(env, escrow_contract);

    let is_paused = is_escrow_contract_paused(env, &client);

    // 1. Escrows where the user is the depositor.
    let mut as_depositor = Vec::new(env);
    let depositor_ids_res = client.try_query_escrows_by_depositor(user, &0, &100);

    if let Ok(Ok(escrows_with_id)) = depositor_ids_res {
        for escrow_with_id in escrows_with_id.iter() {
            let summary = build_summary(
                env,
                &client,
                escrow_with_id.bounty_id,
                &escrow_with_id.escrow,
                is_paused,
            );
            as_depositor.push_back(summary);
        }
    }

    // 2. Escrows where the user is the designated beneficiary/contributor.
    let mut as_beneficiary = Vec::new(env);
    let beneficiary_ids_res = client.try_query_escrows_by_beneficiary(user, &0, &100);

    if let Ok(Ok(escrows_with_id)) = beneficiary_ids_res {
        for escrow_with_id in escrows_with_id.iter() {
            let summary = build_summary(
                env,
                &client,
                escrow_with_id.bounty_id,
                &escrow_with_id.escrow,
                is_paused,
            );
            as_beneficiary.push_back(summary);
        }
    }

    UserPortfolio {
        as_depositor,
        as_beneficiary,
    }
}

/// Query all current delegate assignments for a program escrow.
///
/// If the target program has no active delegate or the query fails for any
/// reason, returns an empty vector to preserve the facade's read-only
/// auditing semantics.
pub fn query_all_delegates(
    env: &Env,
    program_contract: &Address,
    program_id: &String,
) -> Vec<program_escrow::ProgramDelegateInfo> {
    let client = program_escrow::Client::new(env, program_contract);
    let delegates_res = client.try_query_all_delegates(program_id);

    if let Ok(Ok(delegates)) = delegates_res {
        delegates
    } else {
        Vec::new(env)
    }
}

/// Query the payout history for a specific recipient within a program.
///
/// Returns an empty `Vec` if the recipient has never received a payout from
/// this program, including when the underlying query fails.
pub fn query_recipient_history(
    env: &Env,
    program_contract: &Address,
    program_id: &String,
    recipient: &Address,
) -> Vec<program_escrow::PayoutRecord> {
    let client = program_escrow::Client::new(env, program_contract);
    let result = client.try_query_recipient_history(program_id, recipient);

    if let Ok(Ok(records)) = result {
        records
    } else {
        Vec::new(env)
    }
}

/// Fetch [`ProgramData`] for `program_id` on `escrow`, using the
/// per-invocation [`QueryCache`] to avoid redundant cross-contract calls.
pub fn query_program_data_cached(env: &Env, escrow: &Address, program_id: &String) -> ProgramData {
    QueryCache::get_or_load_program_data(env, escrow, program_id)
}

/// Fetch [`FeeConfig`] for `escrow`, using the per-invocation [`QueryCache`].
pub fn query_fee_config_cached(env: &Env, escrow: &Address) -> FeeConfig {
    QueryCache::get_or_load_fee_config(env, escrow)
}

/// Aggregated query returning both [`ProgramData`] and [`FeeConfig`] in a
/// single call, with per-invocation caching for efficiency.
pub fn query_program_balance_and_fee(
    env: &Env,
    escrow: &Address,
    program_id: &String,
) -> (ProgramData, FeeConfig) {
    let data = QueryCache::get_or_load_program_data(env, escrow, program_id);
    let fees = QueryCache::get_or_load_fee_config(env, escrow);
    (data, fees)
}
