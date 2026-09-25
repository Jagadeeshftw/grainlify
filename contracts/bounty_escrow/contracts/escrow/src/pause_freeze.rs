//! Granular pause flags, freeze/unfreeze for escrows and addresses, emergency withdrawal.



use soroban_sdk::{symbol_short, token, Address, Env, String, Symbol, Vec};
use crate::{
    events, rbac, reentrancy_guard,
    DataKey, DeprecationState, DeprecationStatus, Error, Escrow, EscrowStatus,
    FreezeRecord, PauseFlags, PauseStateChanged, RefundRecord,
    events::{emit_deprecation_state_changed, DeprecationStateChanged, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn get_escrow_freeze_record_internal(env: &Env, bounty_id: u64) -> Option<FreezeRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::EscrowFreeze(bounty_id))
}


pub(crate) fn get_address_freeze_record_internal(env: &Env, address: &Address) -> Option<FreezeRecord> {
    env.storage()
        .persistent()
        .get(&DataKey::AddressFreeze(address.clone()))
}


/// # Freeze precedence (escrow-level vs address-level)
///
/// The contract has two **independent** freeze layers:
/// - escrow-level: `EscrowFreeze(bounty_id)` via `freeze_escrow`
/// - address-level: `AddressFreeze(address)` via `freeze_address`,
///   keyed on the escrow **depositor**
///
/// Every funds-out path (release, partial/batch release, refund, claim,
/// authorize_claim, renew, queued-release execution) checks BOTH layers,
/// escrow first, then depositor address:
///
/// * **Either freeze independently blocks** the operation; both layers
///   must be unfrozen for it to proceed.
/// * When **both** apply, the escrow-level check runs first, so the
///   deterministic error is [`Error::EscrowFrozen`], never
///   [`Error::AddressFrozen`].
/// * Unfreezing one layer never touches the other layer's record;
///   `get_escrow_freeze_record` and `get_address_freeze_record` stay
///   independently queryable and accurate.
/// * Address freezes gate the **depositor** only: a frozen payee
///   (contributor / claim recipient) does not block payouts — freeze the
///   escrow itself to stop a payout to a specific recipient.
/// * Freezes gate funds-out only: `lock_funds` and read-only queries are
///   unaffected.
///
/// Covered by the precedence matrix tests in `test_frozen_balance.rs`.
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


/// Check if an operation is paused
pub(crate) fn check_paused(env: &Env, operation: Symbol) -> bool {
    // HARDENING: Maintenance mode supersedes granular pause flags and
    // halts ALL state-mutating operations (lock, release, refund) globally.
    // This is a stronger guarantee than per-operation pause flags:
    // no new state changes can occur while the contract is under maintenance.
    if is_maintenance_mode(env.clone()) {
        return true;
    }

    let flags = get_pause_flags(env);
    // Maintenance mode blocks ALL operations (lock, release, refund).
    if is_maintenance_mode(env.clone()) {
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

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Updates the granular pause state and metadata for the contract.
///
/// # Arguments
/// * `lock` - If Some(true), prevents new escrows from being created.
/// * `release` - If Some(true), prevents payouts to contributors.
/// * `refund` - If Some(true), prevents depositors from reclaiming funds.
/// * `reason` - Optional UTF-8 string describing why the state was changed.
///
/// # Errors
/// Returns `Error::NotInitialized` if the admin has not been set.
/// Returns `Error::Unauthorized` if the caller is not the registered admin.
pub fn set_paused(
    env: Env,
    lock: Option<bool>,
    release: Option<bool>,
    refund: Option<bool>,
    reason: Option<soroban_sdk::String>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let mut flags = get_pause_flags(&env);
    let timestamp = env.ledger().timestamp();

    if reason.is_some() {
        flags.pause_reason = reason.clone();
    }

    if let Some(paused) = lock {
        flags.lock_paused = paused;
        events::emit_pause_state_changed(
            &env,
            PauseStateChanged {
                operation: symbol_short!("lock"),
                paused,
                admin: admin.clone(),
                reason: reason.clone(),
                timestamp,
            },
        );
    }

    if let Some(paused) = release {
        flags.release_paused = paused;
        events::emit_pause_state_changed(
            &env,
            PauseStateChanged {
                operation: symbol_short!("release"),
                paused,
                admin: admin.clone(),
                reason: reason.clone(),
                timestamp,
            },
        );
    }

    if let Some(paused) = refund {
        flags.refund_paused = paused;
        events::emit_pause_state_changed(
            &env,
            PauseStateChanged {
                operation: symbol_short!("refund"),
                paused,
                admin: admin.clone(),
                reason: reason.clone(),
                timestamp,
            },
        );
    }

    let any_paused = flags.lock_paused || flags.release_paused || flags.refund_paused;

    if any_paused {
        if flags.paused_at == 0 {
            flags.paused_at = timestamp;
        }
    } else {
        flags.pause_reason = None;
        flags.paused_at = 0;
    }

    env.storage().instance().set(&DataKey::PauseFlags, &flags);
    Ok(())
}


/// Drains all reward tokens from the contract to a target address.
///
/// This is an emergency recovery function and should only be used as a last resort.
/// The contract MUST have `lock_paused = true` before calling this.
///
/// # Arguments
/// * `target` - The address that will receive the full contract balance.
///
/// # Errors
/// Returns `Error::NotPaused` if `lock_paused` is false.
/// Returns `Error::Unauthorized` if the caller is not the admin.
pub fn emergency_withdraw(env: Env, target: Address) -> Result<(), Error> {
    // GUARD: acquire reentrancy lock
    reentrancy_guard::acquire(&env);

    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    let flags = get_pause_flags(&env);
    if !flags.lock_paused {
        reentrancy_guard::release(&env);
        return Err(Error::NotPaused);
    }

    let token_address: Address = env.storage().instance().get(&DataKey::Token).unwrap();
    let token_client = token::TokenClient::new(&env, &token_address);

    let contract_address = env.current_contract_address();
    let balance = token_client.balance(&contract_address);

    if balance > 0 {
        token_client.transfer(&contract_address, &target, &balance);
        events::emit_emergency_withdraw(
            &env,
            events::EmergencyWithdrawEvent {
                version: EVENT_VERSION_V2,
                admin,
                recipient: target,
                amount: balance,
                timestamp: env.ledger().timestamp(),
            },
        );
    }

    // GUARD: release reentrancy lock
    reentrancy_guard::release(&env);
    Ok(())
}


/// Set deprecation (kill switch) and optional migration target. Admin only.
/// When deprecated is true: new lock_funds and batch_lock_funds are blocked; existing escrows
/// can still release, refund, or be migrated off-chain. Emits DeprecationStateChanged.
pub fn set_deprecated(
    env: Env,
    deprecated: bool,
    migration_target: Option<Address>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let state = DeprecationState {
        deprecated,
        migration_target: migration_target.clone(),
    };
    env.storage()
        .instance()
        .set(&DataKey::DeprecationState, &state);
    emit_deprecation_state_changed(
        &env,
        DeprecationStateChanged {
            deprecated: state.deprecated,
            migration_target: state.migration_target,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


/// View: returns whether the contract is deprecated and the optional migration target address.
pub fn get_deprecation_status(env: Env) -> DeprecationStatus {
    let s = crate::participant_filter::get_deprecation_state(&env);
    DeprecationStatus {
        deprecated: s.deprecated,
        migration_target: s.migration_target,
    }
}


/// Get current pause flags
pub fn get_pause_flags(env: &Env) -> PauseFlags {
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


/// Freeze a specific escrow so release and refund paths fail before any token transfer.
///
/// Read-only queries remain available while the freeze is active.
///
/// # Precedence
/// Independent of any address-level freeze: this blocks the escrow even
/// if its depositor is unfrozen, and unfreezing it does not lift an
/// address-level freeze on the depositor (see `ensure_escrow_not_frozen`
/// for the full precedence rules). When both layers are frozen, this
/// layer's error (`EscrowFrozen`) is the one reported.
pub fn freeze_escrow(
    env: Env,
    bounty_id: u64,
    reason: Option<soroban_sdk::String>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let record = FreezeRecord {
        frozen: true,
        reason,
        frozen_at: env.ledger().timestamp(),
        frozen_by: admin,
    };
    env.storage()
        .persistent()
        .set(&DataKey::EscrowFreeze(bounty_id), &record);
    env.events()
        .publish((symbol_short!("frzesc"), bounty_id), record);
    Ok(())
}


/// Remove an escrow-level freeze and restore normal release/refund behavior.
pub fn unfreeze_escrow(env: Env, bounty_id: u64) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    env.storage()
        .persistent()
        .remove(&DataKey::EscrowFreeze(bounty_id));
    env.events().publish(
        (symbol_short!("unfrzes"), bounty_id),
        (admin, env.ledger().timestamp()),
    );
    Ok(())
}


/// Return the current escrow-level freeze record, if one exists.
pub fn get_escrow_freeze_record(env: Env, bounty_id: u64) -> Option<FreezeRecord> {
    get_escrow_freeze_record_internal(&env, bounty_id)
}


/// Return the escrow data for a given bounty_id. Returns an error if not found.
///
/// Anonymization-aware: only reads `DataKey::Escrow`, never `DataKey::EscrowAnon`,
/// so a bounty locked via `lock_funds_anonymous` returns `BountyNotFound` here rather
/// than any depositor-bearing record. See `docs/anonymous-lock-privacy.md`.
pub fn get_escrow_info(env: Env, bounty_id: u64) -> Result<Escrow, Error> {
    let escrow: Escrow = env
        .storage()
        .persistent()
        .get(&DataKey::Escrow(bounty_id))
        .ok_or(Error::BountyNotFound)?;
    let archival = escrow.archived
        || matches!(escrow.status, EscrowStatus::Released | EscrowStatus::Refunded);
    crate::lock::renew_escrow_record(&env, bounty_id, archival);
    Ok(escrow)
}


/// Compatibility view retained for the independently runnable lifecycle
/// test suite. New callers should prefer `get_escrow_info` so missing
/// records are represented as typed errors.
pub fn get_escrow(env: Env, bounty_id: u64) -> Escrow {
    get_escrow_info(env, bounty_id)
        .unwrap_or_else(|_| panic!("Bounty not found"))
}


/// Return the refund records attached to an escrow for lifecycle tests and
/// legacy clients. Missing bounties remain an explicit contract failure.
pub fn get_refund_history(env: Env, bounty_id: u64) -> Vec<RefundRecord> {
    get_escrow_info(env, bounty_id)
        .unwrap_or_else(|_| panic!("Bounty not found"))
        .refund_history
}


pub fn get_balance(env: Env) -> i128 {
    let token_addr: Address = env
        .storage()
        .instance()
        .get(&DataKey::Token)
        .unwrap_or_else(|| panic!("not initialized"));
    let client = token::Client::new(&env, &token_addr);
    client.balance(&env.current_contract_address())
}


/// Freeze all release/refund operations for escrows owned by `address`.
///
/// Read-only queries remain available while the freeze is active.
///
/// # Precedence
/// `address` is matched against the escrow **depositor** on every
/// funds-out path; freezing a contributor or claim recipient has no
/// blocking effect. Independent of any escrow-level freeze: it blocks
/// all of the depositor's escrows even when none of them is individually
/// frozen, and unfreezing an escrow does not lift this freeze (see
/// `ensure_escrow_not_frozen` for the full precedence rules).
pub fn freeze_address(
    env: Env,
    address: Address,
    reason: Option<soroban_sdk::String>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let record = FreezeRecord {
        frozen: true,
        reason,
        frozen_at: env.ledger().timestamp(),
        frozen_by: admin,
    };
    env.storage()
        .persistent()
        .set(&DataKey::AddressFreeze(address.clone()), &record);
    env.events()
        .publish((symbol_short!("frzaddr"), address), record);
    Ok(())
}


/// Remove an address-level freeze and restore normal release/refund behavior.
pub fn unfreeze_address(env: Env, address: Address) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    env.storage()
        .persistent()
        .remove(&DataKey::AddressFreeze(address.clone()));
    env.events().publish(
        (symbol_short!("unfrzad"), address),
        (admin, env.ledger().timestamp()),
    );
    Ok(())
}


/// Return the current address-level freeze record, if one exists.
pub fn get_address_freeze_record(env: Env, address: Address) -> Option<FreezeRecord> {
    get_address_freeze_record_internal(&env, &address)
}


/// Check if the contract is in maintenance mode
pub fn is_maintenance_mode(env: Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::MaintenanceMode)
        .unwrap_or(false)
}


pub fn get_maintenance_schema_version(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::MaintenanceModeSchemaVersion)
        .unwrap_or(0)
}


/// Update maintenance mode (admin only)
pub fn set_maintenance_mode(
    env: Env,
    enabled: bool,
    reason: Option<String>,
) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let previous_enabled = env
        .storage()
        .instance()
        .get(&DataKey::MaintenanceMode)
        .unwrap_or(false);

    // Idempotent behavior: if no state change, do not emit events.
    if previous_enabled == enabled {
        return Ok(());
    }

    env.storage()
        .instance()
        .set(&DataKey::MaintenanceMode, &enabled);
    env.storage().instance().set(
        &DataKey::MaintenanceModeUpdatedAt,
        &env.ledger().timestamp(),
    );
    env.storage()
        .instance()
        .set(&DataKey::MaintenanceModeUpdatedBy, &admin);

    events::emit_maintenance_mode_changed(
        &env,
        events::MaintenanceModeChanged {
            enabled,
            reason: reason.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );
    events::emit_maintenance_mode_changed_v2(
        &env,
        events::MaintenanceModeChangedV2 {
            version: EVENT_VERSION_V2,
            previous_enabled,
            enabled,
            reason,
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );
    Ok(())
}


// ============================================================================
// CEI + REENTRANCY GUARD HARDENING
// ============================================================================

/// View: Checks if the reentrancy guard is currently active.
pub fn is_reentrancy_guard_locked(env: Env) -> bool {
    env.storage()
        .instance()
        .get(&symbol_short!("r_guard"))
        .unwrap_or(false)
}

