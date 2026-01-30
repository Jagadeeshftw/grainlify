use crate::indexed::indexed_storage::{
    BountyStatus, IndexedBounty, PaginatedResult, QueryFilter, BOUNTY_INDEX, DEPOSITOR_INDEX,
    STATUS_INDEX,
};
use soroban_sdk::{contracttype, Address, Env, Vec};

// ============================================================================
// Main Query Function
// ============================================================================

pub fn query_bounties(
    env: &Env,
    filter: QueryFilter,
    page: u32,
    page_size: u32,
) -> PaginatedResult {
    let mut results = Vec::new(env);
    let mut total_count: u32 = 0;

    let bounties = get_filtered_bounties(env, &filter);
    total_count = bounties.len();

    let start = page * page_size;
    let end = ((page + 1) * page_size).min(total_count);

    for i in start..end {
        if let Some(bounty) = bounties.get(i) {
            results.push_back(bounty);
        }
    }

    PaginatedResult {
        items: results,
        total_count,
        page,
        page_size,
        has_more: end < total_count,
    }
}

// ============================================================================
// Internal Filter Functions
// ============================================================================

fn get_filtered_bounties(env: &Env, filter: &QueryFilter) -> Vec<IndexedBounty> {
    // Optimize by using the most selective index first
    if let status = &filter.status {
        return get_by_status(env, status.clone(), filter);
    }

    if let Some(depositor) = &filter.depositor {
        return get_by_depositor(env, depositor.clone(), filter);
    }

    get_all_filtered(env, filter)
}

fn get_by_status(env: &Env, status: BountyStatus, filter: &QueryFilter) -> Vec<IndexedBounty> {
    let mut results = Vec::new(env);
    let mut bounty_id = 0u64;

    loop {
        let key = (STATUS_INDEX, status.clone(), bounty_id);
        if env.storage().persistent().has(&key) {
            if let Some(bounty) = get_bounty_if_matches(env, bounty_id, filter) {
                results.push_back(bounty);
            }
        }
        bounty_id += 1;
        if bounty_id > 1_000_000 {
            break;
        }
    }

    results
}

fn get_by_depositor(env: &Env, depositor: Address, filter: &QueryFilter) -> Vec<IndexedBounty> {
    let mut results = Vec::new(env);
    let mut bounty_id = 0u64;

    loop {
        let key = (DEPOSITOR_INDEX, depositor.clone(), bounty_id);
        if env.storage().persistent().has(&key) {
            if let Some(bounty) = get_bounty_if_matches(env, bounty_id, filter) {
                results.push_back(bounty);
            }
        }
        bounty_id += 1;
        if bounty_id > 1_000_000 {
            break;
        }
    }

    results
}

fn get_all_filtered(env: &Env, filter: &QueryFilter) -> Vec<IndexedBounty> {
    let mut results = Vec::new(env);

    for bounty_id in 0..1_000_000u64 {
        let key = (BOUNTY_INDEX, bounty_id);
        if let Some(bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
            if matches_filter(&bounty, filter) {
                results.push_back(bounty);
            }
        }
    }

    results
}

fn get_bounty_if_matches(env: &Env, bounty_id: u64, filter: &QueryFilter) -> Option<IndexedBounty> {
    let key = (BOUNTY_INDEX, bounty_id);
    if let Some(bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
        if matches_filter(&bounty, filter) {
            return Some(bounty);
        }
    }
    None
}

fn matches_filter(bounty: &IndexedBounty, filter: &QueryFilter) -> bool {
    if let ref status = filter.status {
        if &bounty.status != status {
            return false;
        }
    }

    if let Some(ref depositor) = filter.depositor {
        if &bounty.depositor != depositor {
            return false;
        }
    }

    if let Some(min_amount) = filter.min_amount {
        if bounty.amount < min_amount {
            return false;
        }
    }

    if let Some(max_amount) = filter.max_amount {
        if bounty.amount > max_amount {
            return false;
        }
    }

    if let Some(from_ts) = filter.from_timestamp {
        if bounty.created_at < from_ts {
            return false;
        }
    }

    if let Some(to_ts) = filter.to_timestamp {
        if bounty.created_at > to_ts {
            return false;
        }
    }

    true
}

// ============================================================================
// Specialized Query Functions
// ============================================================================

pub fn get_bounties_by_status(env: &Env, status: BountyStatus) -> Vec<IndexedBounty> {
    let filter = QueryFilter {
        status: status,
        depositor: None,
        min_amount: None,
        max_amount: None,
        from_timestamp: None,
        to_timestamp: None,
    };
    get_filtered_bounties(env, &filter)
}

pub fn get_bounties_by_depositor(env: &Env, depositor: Address) -> Vec<IndexedBounty> {
    let filter = QueryFilter {
        status: BountyStatus::None,
        depositor: Some(depositor),
        min_amount: None,
        max_amount: None,
        from_timestamp: None,
        to_timestamp: None,
    };
    get_filtered_bounties(env, &filter)
}

pub fn get_bounties_by_amount_range(env: &Env, min: i128, max: i128) -> Vec<IndexedBounty> {
    let filter = QueryFilter {
        status: BountyStatus::None,
        depositor: None,
        min_amount: Some(min),
        max_amount: Some(max),
        from_timestamp: None,
        to_timestamp: None,
    };
    get_filtered_bounties(env, &filter)
}

pub fn get_bounties_by_date_range(env: &Env, from: u64, to: u64) -> Vec<IndexedBounty> {
    let filter = QueryFilter {
        status: BountyStatus::None,
        depositor: None,
        min_amount: None,
        max_amount: None,
        from_timestamp: Some(from),
        to_timestamp: Some(to),
    };
    get_filtered_bounties(env, &filter)
}

pub fn get_recent_bounties(env: &Env, count: u32) -> Vec<IndexedBounty> {
    let mut results = Vec::new(env);
    let mut found = 0u32;

    // Iterate backwards to get most recent first
    for bounty_id in (0..1_000_000u64).rev() {
        let key = (BOUNTY_INDEX, bounty_id);
        if let Some(bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
            results.push_back(bounty);
            found += 1;
            if found >= count {
                break;
            }
        }
    }

    results
}

pub fn get_total_locked_amount(env: &Env) -> i128 {
    let filter = QueryFilter {
        status: BountyStatus::Locked,
        depositor: None,
        min_amount: None,
        max_amount: None,
        from_timestamp: None,
        to_timestamp: None,
    };

    let bounties = get_filtered_bounties(env, &filter);
    let mut total = 0i128;

    for i in 0..bounties.len() {
        if let Some(bounty) = bounties.get(i) {
            total += bounty.amount;
        }
    }

    total
}

// ============================================================================
// Statistics Functions
// ============================================================================

#[derive(Clone, Debug)]
pub struct BountyStats {
    pub locked_count: u32,
    pub released_count: u32,
    pub refunded_count: u32,
    pub partially_released_count: u32,
    pub total_locked: i128,
    pub total_released: i128,
    pub total_refunded: i128,
    pub total_partially_released: i128,
}

pub fn get_bounty_stats(env: &Env) -> BountyStats {
    let mut locked_count = 0u32;
    let mut released_count = 0u32;
    let mut refunded_count = 0u32;
    let mut partially_released_count = 0u32;
    let mut total_locked = 0i128;
    let mut total_released = 0i128;
    let mut total_refunded = 0i128;
    let mut total_partially_released = 0i128;

    for bounty_id in 0..1_000_000u64 {
        let key = (BOUNTY_INDEX, bounty_id);
        if let Some(bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
            match bounty.status {
                BountyStatus::Locked => {
                    locked_count += 1;
                    total_locked += bounty.amount;
                }
                BountyStatus::Released => {
                    released_count += 1;
                    total_released += bounty.amount;
                }
                BountyStatus::Refunded => {
                    refunded_count += 1;
                    total_refunded += bounty.amount;
                }
                BountyStatus::PartiallyReleased => {
                    partially_released_count += 1;
                    total_partially_released += bounty.amount;
                }
                BountyStatus::None => {}
            }
        }
    }

    BountyStats {
        locked_count,
        released_count,
        refunded_count,
        partially_released_count,
        total_locked,
        total_released,
        total_refunded,
        total_partially_released,
    }
}

pub fn get_depositor_stats(env: &Env, depositor: &Address) -> DepositorStats {
    let bounties = get_bounties_by_depositor(env, depositor.clone());

    let mut total_bounties = 0u32;
    let mut active_bounties = 0u32;
    let mut completed_bounties = 0u32;
    let mut total_locked_value = 0i128;
    let mut total_released_value = 0i128;

    for i in 0..bounties.len() {
        if let Some(bounty) = bounties.get(i) {
            total_bounties += 1;

            match bounty.status {
                BountyStatus::Locked => {
                    active_bounties += 1;
                    total_locked_value += bounty.amount;
                }
                BountyStatus::Released => {
                    completed_bounties += 1;
                    total_released_value += bounty.amount;
                }
                BountyStatus::PartiallyReleased => {
                    active_bounties += 1;
                    total_locked_value += bounty.amount;
                }
                BountyStatus::Refunded | BountyStatus::None => {
                    // Refunded bounties don't count as active or completed
                }
            }
        }
    }

    DepositorStats {
        total_bounties,
        active_bounties,
        completed_bounties,
        total_locked_value,
        total_released_value,
    }
}

#[derive(Clone, Debug)]
pub struct DepositorStats {
    pub total_bounties: u32,
    pub active_bounties: u32,
    pub completed_bounties: u32,
    pub total_locked_value: i128,
    pub total_released_value: i128,
}

pub fn get_time_series_data(env: &Env, from: u64, to: u64, interval: u64) -> Vec<TimeSeriesPoint> {
    let mut data_points: Vec<TimeSeriesPoint> = Vec::new(env);
    let mut current = from;

    while current < to {
        let next = current + interval;

        let bounties = get_bounties_by_date_range(env, current, next);
        let mut locked_amount = 0i128;
        let mut released_amount = 0i128;
        let mut count = 0u32;

        for i in 0..bounties.len() {
            if let Some(bounty) = bounties.get(i) {
                count += 1;
                match bounty.status {
                    BountyStatus::Locked | BountyStatus::PartiallyReleased => {
                        locked_amount += bounty.amount;
                    }
                    BountyStatus::Released => {
                        released_amount += bounty.amount;
                    }
                    _ => {}
                }
            }
        }

        data_points.push_back(TimeSeriesPoint {
            timestamp: current,
            count,
            locked_amount,
            released_amount,
        });

        current = next;
    }

    data_points
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub count: u32,
    pub locked_amount: i128,
    pub released_amount: i128,
}
