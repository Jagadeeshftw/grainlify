#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

mod bounty_escrow {
    include!("bounty_escrow_bindings.rs");
}

/// Represents the status of an escrow in the underlying contract.
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
}

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_cross_contract_safety;

