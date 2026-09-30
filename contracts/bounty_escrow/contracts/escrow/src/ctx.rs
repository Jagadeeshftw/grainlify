//! Shared context helpers used across multiple feature modules.
//!
//! These are internal functions extracted from the monolithic lib.rs
//! `impl BountyEscrowContract` block. None are exported in the ABI;
//! they are called via `crate::ctx::*` from feature modules.

use soroban_sdk::{symbol_short, Address, BytesN, Env, Symbol};

use crate::{
    anti_abuse, events, DataKey, Escrow, EscrowStatus, Error, ParticipantFilterMode,
    PauseFlags, DeprecationState, PersistentRecordStatus,
    ARCHIVAL_MARKER_TTL, CLAIM_ARCHIVAL_TTL, CLAIM_LIVE_TTL,
    COMMITMENT_ARCHIVAL_TTL, COMMITMENT_LIVE_TTL, ESCROW_ARCHIVAL_TTL,
    ESCROW_LIVE_TTL, INDEX_ARCHIVAL_TTL, INDEX_LIVE_TTL, TTL_RENEWAL_DIVISOR,
};

// ─── Pause / Maintenance ────────────────────────────────────────────────────

/// Returns `true` when the named operation (`"lock"`, `"release"`, `"refund"`) is blocked.
pub(crate) fn check_paused(env: &Env, operation: Symbol) -> bool {
    // Maintenance mode supersedes granular pause flags.
    if is_maintenance_mode(env) {
        return true;
    }
    let flags = get_pause_flags(env);
    // Second maintenance-mode check retained for parity with original.
    if is_maintenance_mode(env) {
        return true;
    }
    if operation == symbol_short!("lock") {
        return flags.lock_paused;
    } else if operation == symbol_short!("release") {
        return flags.release_paused;
    } else if operation == symbol_short!("refund") {
        return flags.refund_paused;
    }
    false
}

pub(crate) fn get_pause_flags(env: &Env) -> PauseFlags {
    env.storage()
        .instance()
        .get(&DataKey::PauseFlags)
        .unwrap_or(PauseFlags {
            lock_paused: false,
            release_paused: false,
            refund_paused: false,
            pause_reason: None,
            paused_at: 0,
        })
}

pub(crate) fn is_maintenance_mode(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::MaintenanceMode)
        .unwrap_or(false)
}

// ─── Deprecation ────────────────────────────────────────────────────────────

pub(crate) fn get_deprecation_state(env: &Env) -> DeprecationState {
    env.storage()
        .instance()
        .get(&DataKey::DeprecationState)
        .unwrap_or(DeprecationState {
            deprecated: false,
            migration_target: None,
        })
}

// ─── Freeze guards ──────────────────────────────────────────────────────────

pub(crate) fn ensure_escrow_not_frozen(env: &Env, bounty_id: u64) -> Result<(), Error> {
    if get_escrow_freeze_record_internal(env, bounty_id)
        .map(|record| record.frozen)
        .unwrap_or(false)
    {
        return Err(Error::EscrowFrozen);
    }
    Ok(())
}

pub(crate) fn ensure_address_not_frozen(env: &Env, address: &Address) -> Result<(), Error> {
    if get_address_freeze_record_internal(env, address)
        .map(|record| record.frozen)
        .unwrap_or(false)
    {
        return Err(Error::AddressFrozen);
    }
    Ok(())
}

pub(crate) fn get_escrow_freeze_record_internal(
    env: &Env,
    bounty_id: u64,
) -> Option<crate::FreezeRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::EscrowFreeze(bounty_id))
}

pub(crate) fn get_address_freeze_record_internal(
    env: &Env,
    address: &Address,
) -> Option<crate::FreezeRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::AddressFreeze(address.clone()))
}

// ─── Participant filter ──────────────────────────────────────────────────────

pub(crate) fn check_participant_filter(env: &Env, address: Address) -> Result<(), Error> {
    let mode: ParticipantFilterMode = env
        .storage()
        .instance()
        .get(&DataKey::ParticipantFilterMode)
        .unwrap_or(ParticipantFilterMode::Disabled);

    match mode {
        ParticipantFilterMode::Disabled => Ok(()),
        ParticipantFilterMode::BlocklistOnly => {
            if anti_abuse::is_blocklisted(env, address) {
                Err(Error::ParticipantBlocked)
            } else {
                Ok(())
            }
        }
        ParticipantFilterMode::AllowlistOnly => {
            if anti_abuse::is_whitelisted(env, address) {
                Ok(())
            } else {
                Err(Error::ParticipantNotAllowed)
            }
        }
    }
}

// ─── Receipt recording ──────────────────────────────────────────────────────

pub(crate) fn record_receipt(
    _env: &Env,
    _outcome: events::CriticalOperationOutcome,
    _bounty_id: u64,
    _amount: i128,
    _recipient: Address,
) {
    // Backward-compatible no-op until receipt storage/events are fully wired.
}

// ─── TTL renewal ────────────────────────────────────────────────────────────

pub(crate) fn renew_tracked_record(
    env: &Env,
    key: &DataKey,
    marker: &DataKey,
    live_ttl: u32,
    archival_ttl: u32,
    archival: bool,
) {
    if !env.storage().persistent().has(key) {
        return;
    }

    let extension_ttl = if archival { archival_ttl } else { live_ttl };
    let renewal_threshold = extension_ttl / TTL_RENEWAL_DIVISOR;
    let current_ledger = env.ledger().sequence();
    let previous: Option<u32> = env.storage().persistent().get(marker);

    if previous
        .map(|live_until| live_until.saturating_sub(current_ledger) <= renewal_threshold)
        .unwrap_or(true)
    {
        env.storage()
            .persistent()
            .extend_ttl(key, renewal_threshold, extension_ttl);
        env.storage()
            .persistent()
            .set(marker, &current_ledger.saturating_add(extension_ttl));
    }

    env.storage().persistent().extend_ttl(
        marker,
        ARCHIVAL_MARKER_TTL / TTL_RENEWAL_DIVISOR,
        ARCHIVAL_MARKER_TTL,
    );
}

pub(crate) fn renew_escrow_record(env: &Env, bounty_id: u64, archival: bool) {
    let regular = DataKey::Escrow(bounty_id);
    let anonymous = DataKey::EscrowAnon(bounty_id);
    let marker = DataKey::EscrowTtl(bounty_id);
    if env.storage().persistent().has(&regular) {
        let escrow: Option<Escrow> = env.storage().persistent().get(&regular);
        renew_tracked_record(
            env,
            &regular,
            &marker,
            ESCROW_LIVE_TTL,
            ESCROW_ARCHIVAL_TTL,
            archival,
        );
        if archival {
            renew_escrow_index(env, true);
            if let Some(escrow) = escrow {
                renew_depositor_index(env, &escrow.depositor, true);
            }
        }
    } else if env.storage().persistent().has(&anonymous) {
        renew_tracked_record(
            env,
            &anonymous,
            &marker,
            ESCROW_LIVE_TTL,
            ESCROW_ARCHIVAL_TTL,
            archival,
        );
        if archival {
            renew_escrow_index(env, true);
        }
    }
}

pub(crate) fn renew_claim_record(env: &Env, bounty_id: u64, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::PendingClaim(bounty_id),
        &DataKey::ClaimTtl(bounty_id),
        CLAIM_LIVE_TTL,
        CLAIM_ARCHIVAL_TTL,
        archival,
    );
}

pub(crate) fn renew_capability_record(env: &Env, capability_id: &BytesN<32>, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::Capability(capability_id.clone()),
        &DataKey::CapabilityTtl(capability_id.clone()),
        COMMITMENT_LIVE_TTL,
        COMMITMENT_ARCHIVAL_TTL,
        archival,
    );
}

pub(crate) fn renew_escrow_index(env: &Env, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::EscrowIndex,
        &DataKey::EscrowIndexTtl,
        INDEX_LIVE_TTL,
        INDEX_ARCHIVAL_TTL,
        archival,
    );
}

pub(crate) fn renew_depositor_index(env: &Env, depositor: &Address, archival: bool) {
    renew_tracked_record(
        env,
        &DataKey::DepositorIndex(depositor.clone()),
        &DataKey::DepositorIndexTtl(depositor.clone()),
        INDEX_LIVE_TTL,
        INDEX_ARCHIVAL_TTL,
        archival,
    );
}

// ─── Persistent record status ────────────────────────────────────────────────

pub(crate) fn persistent_record_status(
    env: &Env,
    marker: &DataKey,
    record: &DataKey,
) -> PersistentRecordStatus {
    match env.storage().persistent().get::<DataKey, u32>(marker) {
        None if env.storage().persistent().has(record) => PersistentRecordStatus::Live,
        None => PersistentRecordStatus::Missing,
        Some(live_until) if env.ledger().sequence() <= live_until => {
            PersistentRecordStatus::Live
        }
        Some(_) => PersistentRecordStatus::Archived,
    }
}
