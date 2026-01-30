use soroban_sdk::{contracttype, Address, Env, Vec};

// ============================================================================
// Core Data Structures
// ============================================================================

#[contracttype]
#[derive(Clone, Debug)]
pub struct IndexedBounty {
    pub bounty_id: u64,
    pub depositor: Address,
    pub amount: i128,
    pub deadline: u64,
    pub status: BountyStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BountyStatus {
    None,
    Locked,
    Released,
    Refunded,
    PartiallyReleased,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct QueryFilter {
    pub status: BountyStatus,
    pub depositor: Option<Address>,
    pub min_amount: Option<i128>,
    pub max_amount: Option<i128>,
    pub from_timestamp: Option<u64>,
    pub to_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct PaginatedResult {
    pub items: Vec<IndexedBounty>,
    pub total_count: u32,
    pub page: u32,
    pub page_size: u32,
    pub has_more: bool,
}

// ============================================================================
// Storage Index Keys
// ============================================================================

pub const BOUNTY_INDEX: &str = "BIDX";
pub const STATUS_INDEX: &str = "SIDX";
pub const DEPOSITOR_INDEX: &str = "DIDX";
pub const AMOUNT_INDEX: &str = "AIDX";
pub const TIMESTAMP_INDEX: &str = "TIDX";

// ============================================================================
// Index Management Functions
// ============================================================================

/// Indexes a bounty in all relevant indices for efficient querying
pub fn index_bounty(env: &Env, bounty: IndexedBounty) {
    // Primary index: bounty_id -> bounty data
    let key = (BOUNTY_INDEX, bounty.bounty_id);
    env.storage().persistent().set(&key, &bounty);

    // Status index: (status, bounty_id) -> true
    let status_key = (STATUS_INDEX, bounty.status.clone(), bounty.bounty_id);
    env.storage().persistent().set(&status_key, &true);

    // Depositor index: (depositor, bounty_id) -> true
    let depositor_key = (DEPOSITOR_INDEX, bounty.depositor.clone(), bounty.bounty_id);
    env.storage().persistent().set(&depositor_key, &true);

    // Amount index: (amount_bucket, bounty_id) -> true
    // Using buckets to group similar amounts
    let amount_bucket = (bounty.amount / 1_000_000_000) as u64;
    let amount_key = (AMOUNT_INDEX, amount_bucket, bounty.bounty_id);
    env.storage().persistent().set(&amount_key, &true);

    // Timestamp index: (timestamp_bucket, bounty_id) -> true
    // Using daily buckets (86400 seconds = 1 day)
    let timestamp_bucket = bounty.created_at / 86400;
    let timestamp_key = (TIMESTAMP_INDEX, timestamp_bucket, bounty.bounty_id);
    env.storage().persistent().set(&timestamp_key, &true);
}

/// Updates the status of a bounty and re-indexes accordingly
pub fn update_bounty_status(env: &Env, bounty_id: u64, new_status: BountyStatus) {
    let key = (BOUNTY_INDEX, bounty_id);
    if let Some(mut bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
        // Remove old status index entry
        let old_status_key = (STATUS_INDEX, bounty.status.clone(), bounty_id);
        env.storage().persistent().remove(&old_status_key);

        // Update bounty
        bounty.status = new_status.clone();
        bounty.updated_at = env.ledger().timestamp();

        // Save updated bounty
        env.storage().persistent().set(&key, &bounty);

        // Add new status index entry
        let new_status_key = (STATUS_INDEX, new_status, bounty_id);
        env.storage().persistent().set(&new_status_key, &true);
    }
}

/// Retrieves a bounty by its ID
pub fn get_bounty(env: &Env, bounty_id: u64) -> Option<IndexedBounty> {
    let key = (BOUNTY_INDEX, bounty_id);
    env.storage().persistent().get(&key)
}

/// Updates the amount of a bounty
pub fn update_bounty_amount(env: &Env, bounty_id: u64, new_amount: i128) {
    let key = (BOUNTY_INDEX, bounty_id);
    if let Some(mut bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
        // Remove old amount index entry
        let old_amount_bucket = (bounty.amount / 1_000_000_000) as u64;
        let old_amount_key = (AMOUNT_INDEX, old_amount_bucket, bounty_id);
        env.storage().persistent().remove(&old_amount_key);

        // Update bounty
        bounty.amount = new_amount;
        bounty.updated_at = env.ledger().timestamp();

        // Save updated bounty
        env.storage().persistent().set(&key, &bounty);

        // Add new amount index entry
        let new_amount_bucket = (new_amount / 1_000_000_000) as u64;
        let new_amount_key = (AMOUNT_INDEX, new_amount_bucket, bounty_id);
        env.storage().persistent().set(&new_amount_key, &true);
    }
}

/// Updates the deadline of a bounty
pub fn update_bounty_deadline(env: &Env, bounty_id: u64, new_deadline: u64) {
    let key = (BOUNTY_INDEX, bounty_id);
    if let Some(mut bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
        bounty.deadline = new_deadline;
        bounty.updated_at = env.ledger().timestamp();
        env.storage().persistent().set(&key, &bounty);
    }
}

/// Removes a bounty from all indices
pub fn remove_bounty(env: &Env, bounty_id: u64) {
    let key = (BOUNTY_INDEX, bounty_id);
    if let Some(bounty) = env.storage().persistent().get::<_, IndexedBounty>(&key) {
        // Remove from all indices
        let status_key = (STATUS_INDEX, bounty.status.clone(), bounty_id);
        env.storage().persistent().remove(&status_key);

        let depositor_key = (DEPOSITOR_INDEX, bounty.depositor.clone(), bounty_id);
        env.storage().persistent().remove(&depositor_key);

        let amount_bucket = (bounty.amount / 1_000_000_000) as u64;
        let amount_key = (AMOUNT_INDEX, amount_bucket, bounty_id);
        env.storage().persistent().remove(&amount_key);

        let timestamp_bucket = bounty.created_at / 86400;
        let timestamp_key = (TIMESTAMP_INDEX, timestamp_bucket, bounty_id);
        env.storage().persistent().remove(&timestamp_key);

        // Remove primary entry
        env.storage().persistent().remove(&key);
    }
}

/// Checks if a bounty exists
pub fn bounty_exists(env: &Env, bounty_id: u64) -> bool {
    let key = (BOUNTY_INDEX, bounty_id);
    env.storage().persistent().has(&key)
}

/// Gets all bounty IDs for a specific depositor
pub fn get_depositor_bounty_ids(env: &Env, depositor: &Address) -> Vec<u64> {
    let mut bounty_ids = Vec::new(env);

    for bounty_id in 0..1_000_000u64 {
        let key = (DEPOSITOR_INDEX, depositor.clone(), bounty_id);
        if env.storage().persistent().has(&key) {
            bounty_ids.push_back(bounty_id);
        }
    }

    bounty_ids
}

/// Gets all bounty IDs with a specific status
pub fn get_status_bounty_ids(env: &Env, status: &BountyStatus) -> Vec<u64> {
    let mut bounty_ids = Vec::new(env);

    for bounty_id in 0..1_000_000u64 {
        let key = (STATUS_INDEX, status.clone(), bounty_id);
        if env.storage().persistent().has(&key) {
            bounty_ids.push_back(bounty_id);
        }
    }

    bounty_ids
}
