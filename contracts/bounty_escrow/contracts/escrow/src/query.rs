//! Bounded, read-only query entrypoints over escrow state.
//!
//! Backed by secondary indexes maintained at write time (see `index.rs`):
//!
//! * `query_escrows_by_status` / `get_escrow_ids_by_status` read
//!   `DataKey::StatusIndex(status)` — O(page size) storage reads.
//! * `query_escrows_by_depositor` reads `DataKey::DepositorIndex(addr)` —
//!   O(page size) storage reads.
//! * `query_escrows_by_amount` / `query_escrows_by_deadline` have no
//!   dedicated index. They scan `DataKey::EscrowIndex` but are hard-bounded
//!   by [`MAX_FILTER_SCAN_ENTRIES`] record reads per call and stop as soon as
//!   the requested page window is filled. Worst-case cost is therefore
//!   O(`MAX_FILTER_SCAN_ENTRIES`); for very large datasets callers should
//!   narrow candidates with the status or depositor queries first and apply
//!   range filters per page.
//!
//! All entrypoints use offset/limit pagination with a hard page-size cap
//! ([`MAX_QUERY_PAGE_SIZE`]), matching the participant-filter conventions:
//! `limit == 0` or an out-of-range `offset` yields an empty page.

use crate::{index, DataKey, Escrow, EscrowStatus, EscrowWithId};
use soroban_sdk::{Address, Env, Vec};

/// Maximum number of records returned per query call. Matches the
/// participant-filter page cap; larger `limit` values are silently clamped.
pub(crate) const MAX_QUERY_PAGE_SIZE: u32 = 50;

/// Hard bound on `EscrowIndex` entries examined by the amount/deadline
/// range scans. Keeps every query call bounded in ledger resources
/// regardless of total escrow count.
const MAX_FILTER_SCAN_ENTRIES: u32 = 512;

/// Returns `Some((offset, clamped_limit))`, or `None` when the requested
/// page is empty by definition (`limit == 0`).
fn page_window(offset: u32, limit: u32) -> Option<(u32, u32)> {
    let limit = limit.min(MAX_QUERY_PAGE_SIZE);
    if limit == 0 {
        return None;
    }
    Some((offset, limit))
}

fn load_with_id(env: &Env, bounty_id: u64) -> Option<EscrowWithId> {
    env.storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        .map(|escrow| EscrowWithId { bounty_id, escrow })
}

/// Slice `ids` to `[offset, offset + limit)` without materializing the tail.
fn slice_ids(env: &Env, ids: &Vec<u64>, offset: u32, limit: u32) -> Vec<u64> {
    let mut out = Vec::new(env);
    let start = offset.min(ids.len());
    let take = limit.min(ids.len() - start);
    for i in 0..take {
        if let Some(id) = ids.get(start + i) {
            out.push_back(id);
        }
    }
    out
}

/// Paginated bounty ids for a single escrow status.
pub(crate) fn status_ids(
    env: &Env,
    status: EscrowStatus,
    offset: u32,
    limit: u32,
) -> Vec<u64> {
    let Some((offset, limit)) = page_window(offset, limit) else {
        return Vec::new(env);
    };
    let ids = index::read_status_ids(env, status);
    slice_ids(env, &ids, offset, limit)
}

/// `query_escrows_by_status` — paginated escrows filtered by status.
pub(crate) fn query_by_status(
    env: &Env,
    status: EscrowStatus,
    offset: u32,
    limit: u32,
) -> Vec<EscrowWithId> {
    let mut out = Vec::new(env);
    let Some((offset, limit)) = page_window(offset, limit) else {
        return out;
    };
    let ids = index::read_status_ids(env, status);
    for id in slice_ids(env, &ids, offset, limit).iter() {
        // Skip orphaned index entries (e.g. records removed out-of-band).
        if let Some(item) = load_with_id(env, id) {
            out.push_back(item);
        }
    }
    out
}

/// `get_escrow_ids_by_status` — paginated bounty ids for a status.
pub(crate) fn ids_by_status(env: &Env, status: EscrowStatus, offset: u32, limit: u32) -> Vec<u64> {
    status_ids(env, status, offset, limit)
}

/// `query_escrows_by_depositor` — paginated escrows for one depositor.
pub(crate) fn query_by_depositor(
    env: &Env,
    depositor: &Address,
    offset: u32,
    limit: u32,
) -> Vec<EscrowWithId> {
    let mut out = Vec::new(env);
    let Some((_offset, _limit)) = page_window(offset, limit) else {
        return out;
    };
    let ids = index::read_depositor_ids(env, depositor);
    for id in slice_ids(env, &ids, offset, limit).iter() {
        if let Some(item) = load_with_id(env, id) {
            out.push_back(item);
        }
    }
    out
}

/// Shared bounded scan over `EscrowIndex` for range filters.
///
/// Collects ids whose stored escrow satisfies `keep` until the page window
/// `[offset, offset + limit)` is filled or `MAX_FILTER_SCAN_ENTRIES` index
/// entries have been examined, whichever comes first.
fn scan_filtered(
    env: &Env,
    offset: u32,
    limit: u32,
    keep: impl Fn(&Escrow) -> bool,
) -> Vec<EscrowWithId> {
    let mut out = Vec::new(env);
    let Some((offset, limit)) = page_window(offset, limit) else {
        return out;
    };
    let index_ids = env
        .storage()
        .persistent()
        .get::<DataKey, Vec<u64>>(&DataKey::EscrowIndex)
        .unwrap_or(Vec::new(env));

    let needed = offset as u64 + limit as u64;
    let mut matches_seen = 0u64;
    let mut scanned = 0u32;

    for id in index_ids.iter() {
        if scanned >= MAX_FILTER_SCAN_ENTRIES || matches_seen >= needed {
            break;
        }
        scanned += 1;
        // Anonymous escrows and orphaned entries have no `Escrow` record.
        let Some(escrow) = env
            .storage()
            .persistent()
            .get::<DataKey, Escrow>(&DataKey::Escrow(id))
        else {
            continue;
        };
        if !keep(&escrow) {
            continue;
        }
        matches_seen += 1;
        if matches_seen > offset as u64 {
            out.push_back(EscrowWithId {
                bounty_id: id,
                escrow,
            });
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// `query_escrows_by_amount` — paginated escrows with `min <= amount <= max`.
pub(crate) fn query_by_amount(
    env: &Env,
    min_amount: i128,
    max_amount: i128,
    offset: u32,
    limit: u32,
) -> Vec<EscrowWithId> {
    scan_filtered(env, offset, limit, |escrow| {
        escrow.amount >= min_amount && escrow.amount <= max_amount
    })
}

/// `query_escrows_by_deadline` — paginated escrows with
/// `min_deadline <= deadline <= max_deadline`.
pub(crate) fn query_by_deadline(
    env: &Env,
    min_deadline: u64,
    max_deadline: u64,
    offset: u32,
    limit: u32,
) -> Vec<EscrowWithId> {
    scan_filtered(env, offset, limit, |escrow| {
        escrow.deadline >= min_deadline && escrow.deadline <= max_deadline
    })
}
