//! Secondary indexes backing the bounded escrow query entrypoints.
//!
//! Two index families are maintained at write time:
//!
//! * `DataKey::StatusIndex(discriminant)` — `Vec<u64>` of bounty ids per
//!   [`EscrowStatus`], covering regular (non-anonymous) escrows only.
//! * `DataKey::DepositorIndex(address)` — `Vec<u64>` of bounty ids per
//!   depositor, maintained inline by `lock.rs` / `renewal.rs`.
//!
//! Anonymous escrows (`DataKey::EscrowAnon`) are intentionally excluded: the
//! query entrypoints return [`crate::EscrowWithId`], which cannot represent a
//! commitment-only depositor.

use crate::{DataKey, Escrow, EscrowStatus, INDEX_LIVE_TTL, TTL_RENEWAL_DIVISOR};
use soroban_sdk::{Env, Vec};

/// Map an [`EscrowStatus`] to the stable `u32` discriminant used inside
/// [`DataKey::StatusIndex`]. These values are part of the on-disk layout;
/// they must not change without a storage migration.
pub(crate) fn status_discriminant(status: EscrowStatus) -> u32 {
    match status {
        EscrowStatus::Draft => 0,
        EscrowStatus::Locked => 1,
        EscrowStatus::Released => 2,
        EscrowStatus::Refunded => 3,
        EscrowStatus::PartiallyRefunded => 4,
    }
}

pub(crate) fn status_index_key(status: EscrowStatus) -> DataKey {
    DataKey::StatusIndex(status_discriminant(status))
}

fn read_ids(env: &Env, key: &DataKey) -> Vec<u64> {
    env.storage()
        .persistent()
        .get(key)
        .unwrap_or(Vec::new(env))
}

fn write_ids(env: &Env, key: &DataKey, ids: &Vec<u64>) {
    env.storage().persistent().set(key, ids);
    env.storage().persistent().extend_ttl(
        key,
        INDEX_LIVE_TTL / TTL_RENEWAL_DIVISOR,
        INDEX_LIVE_TTL,
    );
}

fn add_id(env: &Env, key: &DataKey, bounty_id: u64) {
    let mut ids = read_ids(env, key);
    // Defensive dedup: an id must appear at most once per index bucket.
    for existing in ids.iter() {
        if existing == bounty_id {
            return;
        }
    }
    ids.push_back(bounty_id);
    write_ids(env, key, &ids);
}

fn remove_id(env: &Env, key: &DataKey, bounty_id: u64) {
    let ids = read_ids(env, key);
    let mut kept = Vec::new(env);
    for id in ids.iter() {
        if id != bounty_id {
            kept.push_back(id);
        }
    }
    write_ids(env, key, &kept);
}

/// Read the bounty ids indexed under `status`, renewing the index TTL.
pub(crate) fn read_status_ids(env: &Env, status: EscrowStatus) -> Vec<u64> {
    let key = status_index_key(status);
    let ids = read_ids(env, &key);
    if !ids.is_empty() {
        env.storage().persistent().extend_ttl(
            &key,
            INDEX_LIVE_TTL / TTL_RENEWAL_DIVISOR,
            INDEX_LIVE_TTL,
        );
    }
    ids
}

/// Read the bounty ids indexed for `depositor`, renewing the index TTL.
pub(crate) fn read_depositor_ids(env: &Env, depositor: &soroban_sdk::Address) -> Vec<u64> {
    let key = DataKey::DepositorIndex(depositor.clone());
    let ids = read_ids(env, &key);
    if !ids.is_empty() {
        env.storage().persistent().extend_ttl(
            &key,
            INDEX_LIVE_TTL / TTL_RENEWAL_DIVISOR,
            INDEX_LIVE_TTL,
        );
    }
    ids
}

/// Reconcile the status index with an escrow write.
///
/// Must be invoked **before** the new [`Escrow`] value is persisted under
/// `DataKey::Escrow(bounty_id)`, because it reads the previously stored
/// record to determine which index bucket is being left. When no record
/// exists yet the id is added to the `new_status` bucket (creation path).
/// Writes that do not change `status` are a no-op.
pub(crate) fn sync_status(env: &Env, bounty_id: u64, new_status: EscrowStatus) {
    let prev = env
        .storage()
        .persistent()
        .get::<DataKey, Escrow>(&DataKey::Escrow(bounty_id))
        .map(|escrow| escrow.status);

    if let Some(prev_status) = prev {
        if prev_status != new_status {
            remove_id(env, &status_index_key(prev_status), bounty_id);
        }
    }
    add_id(env, &status_index_key(new_status), bounty_id);
}
