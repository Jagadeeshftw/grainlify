#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

use program_escrow::{FeeConfig, ProgramData};

mod bounty_escrow {
    include!("bounty_escrow_bindings.rs");
}

mod program_escrow {
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
        if let Some(cached) = env.storage().temporary().get::<QueryCacheKey, ProgramData>(&key) {
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
        if let Some(cached) = env.storage().temporary().get::<QueryCacheKey, FeeConfig>(&key) {
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
/// Must match `EscrowStatus` in BountyEscrow.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
    PartiallyRefunded,
}

/// A simplified summary of an escrow designed for frontend consumption.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EscrowSummary {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub remaining_amount: i128,
    pub status: EscrowStatus,
    pub deadline: u64,
    pub repo_id: u64,
    pub issue_id: u64,
    pub bounty_type: String,
    pub is_paused: bool,
}

/// A user's aggregated portfolio showing escrows they funded and escrows
/// where they are listed as a beneficiary (if applicable).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserPortfolio {
    /// Escrows funded by this user
    pub as_depositor: Vec<EscrowSummary>,
    /// Escrows where this user is the designated beneficiary/contributor
    pub as_beneficiary: Vec<EscrowSummary>,
}

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
        let client = bounty_escrow::Client::new(&env, &escrow_contract);

        // Retrieve the escrow info. We use try_ to avoid trapping the WASM
        // execution if the bounty does not exist.
        let escrow_info_res = client.try_get_escrow_info(&bounty_id);

        if let Ok(Ok(info)) = escrow_info_res {
            // Retrieve metadata
            let metadata_res = client.try_get_metadata(&bounty_id);

            let (repo_id, issue_id, bounty_type) = if let Ok(Ok(meta)) = metadata_res {
                (meta.repo_id, meta.issue_id, meta.bounty_type)
            } else {
                (0, 0, String::from_str(&env, ""))
            };

            // Map the imported EscrowStatus to our facade's EscrowStatus
            let status = match info.status {
                bounty_escrow::EscrowStatus::Locked => EscrowStatus::Locked,
                bounty_escrow::EscrowStatus::Released => EscrowStatus::Released,
                bounty_escrow::EscrowStatus::Refunded => EscrowStatus::Refunded,
                bounty_escrow::EscrowStatus::PartiallyRefunded => EscrowStatus::PartiallyRefunded,
            };

            // Check if the contract is paused
            let pause_flags_res = client.try_get_pause_flags();
            let is_paused = if let Ok(Ok(flags)) = pause_flags_res {
                flags.lock_paused || flags.release_paused || flags.refund_paused
            } else {
                false
            };
            
            // Map the imported AnonymousParty (since EscrowInfo returns depositor which is AnonymousParty)
            // Note: `get_escrow_info` returns `Escrow` which has `depositor: Address` directly
            
            Some(EscrowSummary {
                bounty_id,
                depositor: info.depositor,
                amount: info.amount,
                remaining_amount: info.remaining_amount,
                status,
                deadline: info.deadline,
                repo_id,
                issue_id,
                bounty_type,
                is_paused,
            })
        } else {
            None
        }
    }

    /// Retrieve summaries for a batch of `bounty_ids`.
    /// Missing escrows are omitted from the result vector.
    pub fn get_escrow_summaries(
        env: Env,
        escrow_contract: Address,
        bounty_ids: Vec<u64>,
    ) -> Vec<EscrowSummary> {
        let mut summaries = Vec::new(&env);

        let client = bounty_escrow::Client::new(&env, &escrow_contract);
        
        let pause_flags_res = client.try_get_pause_flags();
        let is_paused = if let Ok(Ok(flags)) = pause_flags_res {
            flags.lock_paused || flags.release_paused || flags.refund_paused
        } else {
            false
        };

        for id in bounty_ids.iter() {
            let escrow_info_res = client.try_get_escrow_info(&id);
            if let Ok(Ok(info)) = escrow_info_res {
                let metadata_res = client.try_get_metadata(&id);
                let (repo_id, issue_id, bounty_type) = if let Ok(Ok(meta)) = metadata_res {
                    (meta.repo_id, meta.issue_id, meta.bounty_type)
                } else {
                    (0, 0, String::from_str(&env, ""))
                };
                
                 let status = match info.status {
                    bounty_escrow::EscrowStatus::Locked => EscrowStatus::Locked,
                    bounty_escrow::EscrowStatus::Released => EscrowStatus::Released,
                    bounty_escrow::EscrowStatus::Refunded => EscrowStatus::Refunded,
                    bounty_escrow::EscrowStatus::PartiallyRefunded => EscrowStatus::PartiallyRefunded,
                };

                summaries.push_back(EscrowSummary {
                    bounty_id: id,
                    depositor: info.depositor,
                    amount: info.amount,
                    remaining_amount: info.remaining_amount,
                    status,
                    deadline: info.deadline,
                    repo_id,
                    issue_id,
                    bounty_type,
                    is_paused,
                });
            }
        }
        summaries
    }

    /// Retrieve an aggregated view of a user's portolio, including both
    /// the escrows they deposited into and escrows they are listed to receive.
    pub fn get_user_portfolio(
        env: Env,
        escrow_contract: Address,
        user: Address,
    ) -> UserPortfolio {
        let client = bounty_escrow::Client::new(&env, &escrow_contract);

        // 1. Get escrows where user is depositor
        let mut as_depositor = Vec::new(&env);
        let depositor_ids_res = client.try_query_escrows_by_depositor(&user, &0, &100);
        
        // Optimize: Fetch pause flags once
        let pause_flags_res = client.try_get_pause_flags();
        let is_paused = if let Ok(Ok(flags)) = pause_flags_res {
            flags.lock_paused || flags.release_paused || flags.refund_paused
        } else {
            false
        };

        if let Ok(Ok(escrows_with_id)) = depositor_ids_res {
            for escrow_with_id in escrows_with_id.iter() {
                let id = escrow_with_id.bounty_id;
                let info = escrow_with_id.escrow;
                
                let metadata_res = client.try_get_metadata(&id);
                let (repo_id, issue_id, bounty_type) = if let Ok(Ok(meta)) = metadata_res {
                    (meta.repo_id, meta.issue_id, meta.bounty_type)
                } else {
                    (0, 0, String::from_str(&env, ""))
                };

                let status = match info.status {
                    bounty_escrow::EscrowStatus::Locked => EscrowStatus::Locked,
                    bounty_escrow::EscrowStatus::Released => EscrowStatus::Released,
                    bounty_escrow::EscrowStatus::Refunded => EscrowStatus::Refunded,
                    bounty_escrow::EscrowStatus::PartiallyRefunded => EscrowStatus::PartiallyRefunded,
                };

                as_depositor.push_back(EscrowSummary {
                    bounty_id: id,
                    depositor: info.depositor,
                    amount: info.amount,
                    remaining_amount: info.remaining_amount,
                    status,
                    deadline: info.deadline,
                    repo_id,
                    issue_id,
                    bounty_type,
                    is_paused,
                });
            }
        }

        // 2. Setup standard user beneficiary functionality (tickets)
        let as_beneficiary = Vec::new(&env);

        UserPortfolio {
            as_depositor,
            as_beneficiary,
        }
    }

    /// Query all current delegate assignments for a program escrow.
    ///
    /// Returns a list of delegate audit records for the requested program. If
    /// the target program has no active delegate or the query fails for any
    /// reason, this function returns an empty vector to preserve the facade's
    /// read-only auditing semantics.
    ///
    /// **Atomicity Guarantee**: This query reads directly from the underlying
    /// contract in the same transaction. If a delegate was revoked via
    /// `emergency_revoke_delegate`, it will be instantly omitted from these
    /// results, with no caching or stale-read window.
    pub fn query_all_delegates(
        env: Env,
        program_contract: Address,
        program_id: String,
    ) -> Vec<program_escrow::ProgramDelegateInfo> {
        let client = program_escrow::Client::new(&env, &program_contract);
        let delegates_res = client.try_query_all_delegates(&program_id);

        if let Ok(Ok(delegates)) = delegates_res {
            delegates
        } else {
            Vec::new(&env)
        }
    }

    // ========================================================================
    // Cached Query Methods
    // ========================================================================

    /// Fetch [`ProgramData`] for `program_id` on `escrow`, using the
    /// per-invocation [`QueryCache`] to avoid redundant cross-contract calls.
    pub fn query_program_data_cached(
        env: Env,
        escrow: Address,
        program_id: String,
    ) -> ProgramData {
        QueryCache::get_or_load_program_data(&env, &escrow, &program_id)
    }

    /// Fetch [`FeeConfig`] for `escrow`, using the per-invocation [`QueryCache`].
    pub fn query_fee_config_cached(env: Env, escrow: Address) -> FeeConfig {
        QueryCache::get_or_load_fee_config(&env, &escrow)
    }

    /// Aggregated query returning both [`ProgramData`] and [`FeeConfig`] in a
    /// single call, with per-invocation caching for efficiency.
    pub fn query_program_balance_and_fee(
        env: Env,
        escrow: Address,
        program_id: String,
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

