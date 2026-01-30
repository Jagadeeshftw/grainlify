use crate::indexed::indexed_storage::{BountyStatus, IndexedBounty, PaginatedResult, QueryFilter};
use crate::indexed::query_functions;
use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

#[contract]
pub struct EscrowQueryContract;

#[contractimpl]
impl EscrowQueryContract {
    /// Query bounties with flexible filtering and pagination
    pub fn query_bounties(
        env: Env,
        filter: QueryFilter,
        page: u32,
        page_size: u32,
    ) -> PaginatedResult {
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get a single bounty by ID
    pub fn get_bounty(env: Env, bounty_id: u64) -> Option<IndexedBounty> {
        crate::indexed::indexed_storage::get_bounty(&env, bounty_id)
    }

    /// Get all bounties with a specific status
    pub fn get_bounties_by_status(env: Env, status: BountyStatus) -> Vec<IndexedBounty> {
        query_functions::get_bounties_by_status(&env, status)
    }

    /// Get all bounties created by a specific depositor
    pub fn get_bounties_by_depositor(env: Env, depositor: Address) -> Vec<IndexedBounty> {
        query_functions::get_bounties_by_depositor(&env, depositor)
    }

    /// Get bounties within an amount range
    pub fn get_bounties_by_amount_range(env: Env, min: i128, max: i128) -> Vec<IndexedBounty> {
        query_functions::get_bounties_by_amount_range(&env, min, max)
    }

    /// Get bounties created within a date range
    pub fn get_bounties_by_date_range(env: Env, from: u64, to: u64) -> Vec<IndexedBounty> {
        query_functions::get_bounties_by_date_range(&env, from, to)
    }

    /// Get the most recent bounties
    pub fn get_recent_bounties(env: Env, count: u32) -> Vec<IndexedBounty> {
        query_functions::get_recent_bounties(&env, count)
    }

    /// Get total amount currently locked in all bounties
    pub fn get_total_locked_amount(env: Env) -> i128 {
        query_functions::get_total_locked_amount(&env)
    }

    /// Get bounties for a specific depositor with pagination
    pub fn get_depositor_bounties(
        env: Env,
        depositor: Address,
        page: u32,
        page_size: u32,
    ) -> PaginatedResult {
        let filter = QueryFilter {
            status: BountyStatus::None,
            depositor: Some(depositor),
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get all active (locked) bounties with pagination
    pub fn get_active_bounties(env: Env, page: u32, page_size: u32) -> PaginatedResult {
        let filter = QueryFilter {
            status: BountyStatus::Locked,
            depositor: None,
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get all completed (released) bounties with pagination
    pub fn get_completed_bounties(env: Env, page: u32, page_size: u32) -> PaginatedResult {
        let filter = QueryFilter {
            status: BountyStatus::Released,
            depositor: None,
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get all refunded bounties with pagination
    pub fn get_refunded_bounties(env: Env, page: u32, page_size: u32) -> PaginatedResult {
        let filter = QueryFilter {
            status: BountyStatus::Refunded,
            depositor: None,
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get all partially released bounties with pagination
    pub fn get_partially_released_bounties(env: Env, page: u32, page_size: u32) -> PaginatedResult {
        let filter = QueryFilter {
            status: BountyStatus::PartiallyReleased,
            depositor: None,
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get expired bounties (deadline has passed but still locked)
    pub fn get_expired_bounties(env: Env, page: u32, page_size: u32) -> PaginatedResult {
        let current_time = env.ledger().timestamp();
        let mut results = Vec::new(&env);
        let mut total = 0u32;

        for bounty_id in 0..1_000_000u64 {
            if let Some(bounty) = crate::indexed::indexed_storage::get_bounty(&env, bounty_id) {
                if bounty.status == BountyStatus::Locked && bounty.deadline < current_time {
                    total += 1;
                    let start = page * page_size;
                    let end = (page + 1) * page_size;
                    if total > start && total <= end {
                        results.push_back(bounty);
                    }
                }
            }
        }

        PaginatedResult {
            items: results,
            total_count: total,
            page,
            page_size,
            has_more: total > (page + 1) * page_size,
        }
    }

    /// Get bounties expiring soon (within specified seconds)
    pub fn get_expiring_soon_bounties(
        env: Env,
        within_seconds: u64,
        page: u32,
        page_size: u32,
    ) -> PaginatedResult {
        let current_time = env.ledger().timestamp();
        let expiry_threshold = current_time + within_seconds;
        let mut results = Vec::new(&env);
        let mut total = 0u32;

        for bounty_id in 0..1_000_000u64 {
            if let Some(bounty) = crate::indexed::indexed_storage::get_bounty(&env, bounty_id) {
                if bounty.status == BountyStatus::Locked
                    && bounty.deadline > current_time
                    && bounty.deadline <= expiry_threshold
                {
                    total += 1;
                    let start = page * page_size;
                    let end = (page + 1) * page_size;
                    if total > start && total <= end {
                        results.push_back(bounty);
                    }
                }
            }
        }

        PaginatedResult {
            items: results,
            total_count: total,
            page,
            page_size,
            has_more: total > (page + 1) * page_size,
        }
    }

    /// Search bounties with multiple filters
    pub fn search_bounties(
        env: Env,
        depositor: Option<Address>,
        min_amount: Option<i128>,
        max_amount: Option<i128>,
        status: BountyStatus,
        from_date: Option<u64>,
        to_date: Option<u64>,
        page: u32,
        page_size: u32,
    ) -> PaginatedResult {
        let filter = QueryFilter {
            status,
            depositor,
            min_amount,
            max_amount,
            from_timestamp: from_date,
            to_timestamp: to_date,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }

    /// Get total value locked (TVL) across all bounties
    pub fn get_total_value_locked(env: Env) -> i128 {
        query_functions::get_total_locked_amount(&env)
    }

    /// Get count of bounties by status
    pub fn get_bounty_count_by_status(env: Env, status: BountyStatus) -> u32 {
        let bounties = query_functions::get_bounties_by_status(&env, status);
        bounties.len()
    }

    /// Check if a bounty exists
    pub fn bounty_exists(env: Env, bounty_id: u64) -> bool {
        crate::indexed::indexed_storage::get_bounty(&env, bounty_id).is_some()
    }

    /// Get bounties by depositor with status filter
    pub fn get_depositor_bounties_by_status(
        env: Env,
        depositor: Address,
        status: BountyStatus,
        page: u32,
        page_size: u32,
    ) -> PaginatedResult {
        let filter = QueryFilter {
            status: status,
            depositor: Some(depositor),
            min_amount: None,
            max_amount: None,
            from_timestamp: None,
            to_timestamp: None,
        };
        query_functions::query_bounties(&env, filter, page, page_size)
    }
}
