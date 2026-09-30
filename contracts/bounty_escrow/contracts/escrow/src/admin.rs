//! Contract initialisation, admin rotation, maintenance mode, batch size caps, and network identity.



use soroban_sdk::{symbol_short, Address, BytesN, Env, Vec};
use crate::{
    events, rbac,
    AdminRotationConfig, AdminRotationStatus, BatchSizeCaps, DataKey, Error,
    EscrowMetadata, LockFundsItem, PersistentRecordStatus, ReleaseFundsItem,
    ADMIN_TIMELOCK, DEFAULT_ADMIN_ROTATION_TIMELOCK, MAX_ADMIN_ROTATION_TIMELOCK,
    MAX_BATCH_SIZE, MIN_ADMIN_ROTATION_TIMELOCK,
    ESCROW_LIVE_TTL, ESCROW_ARCHIVAL_TTL, CLAIM_LIVE_TTL, CLAIM_ARCHIVAL_TTL,
    COMMITMENT_LIVE_TTL, COMMITMENT_ARCHIVAL_TTL, INDEX_LIVE_TTL, INDEX_ARCHIVAL_TTL,
    ARCHIVAL_MARKER_TTL, TTL_RENEWAL_DIVISOR,
    NOTIFICATION_PREFS_MASK, PARTICIPANT_LIST_SCHEMA_VERSION_V1,
    MAINTENANCE_MODE_SCHEMA_VERSION_V1, FEE_ROUTING_SCHEMA_VERSION_V1,
    HIGH_VALUE_CONFIG_SCHEMA_VERSION_V1, REFUND_ELIGIBILITY_SCHEMA_VERSION_V1,
    Escrow, EscrowStatus,
    events::{emit_admin_rotation_accepted, emit_admin_rotation_cancelled,
             emit_admin_rotation_proposed, emit_admin_rotation_timelock_updated,
             EVENT_VERSION_V2},
    Capability,
};

// ─────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────


pub(crate) fn order_batch_lock_items(env: &Env, items: &Vec<LockFundsItem>) -> Vec<LockFundsItem> {
    let mut ordered: Vec<LockFundsItem> = Vec::new(env);
    for item in items.iter() {
        let mut next: Vec<LockFundsItem> = Vec::new(env);
        let mut inserted = false;
        for existing in ordered.iter() {
            if !inserted && item.bounty_id < existing.bounty_id {
                next.push_back(item.clone());
                inserted = true;
            }
            next.push_back(existing);
        }
        if !inserted {
            next.push_back(item.clone());
        }
        ordered = next;
    }
    ordered
}


pub(crate) fn order_batch_release_items(
    env: &Env,
    items: &Vec<ReleaseFundsItem>,
) -> Vec<ReleaseFundsItem> {
    let mut ordered: Vec<ReleaseFundsItem> = Vec::new(env);
    for item in items.iter() {
        let mut next: Vec<ReleaseFundsItem> = Vec::new(env);
        let mut inserted = false;
        for existing in ordered.iter() {
            if !inserted && item.bounty_id < existing.bounty_id {
                next.push_back(item.clone());
                inserted = true;
            }
            next.push_back(existing);
        }
        if !inserted {
            next.push_back(item.clone());
        }
        ordered = next;
    }
    ordered
}


/// Returns the effective batch size caps, defaulting to the compile-time hard limit.
pub(crate) fn get_batch_size_caps_internal(env: &Env) -> BatchSizeCaps {
    env.storage()
        .instance()
        .get(&DataKey::BatchSizeCaps)
        .unwrap_or(BatchSizeCaps {
            lock_cap: MAX_BATCH_SIZE,
            release_cap: MAX_BATCH_SIZE,
        })
}


pub(crate) fn validate_batch_size_caps(caps: &BatchSizeCaps) -> Result<(), Error> {
    if caps.lock_cap == 0
        || caps.release_cap == 0
        || caps.lock_cap > MAX_BATCH_SIZE
        || caps.release_cap > MAX_BATCH_SIZE
    {
        return Err(Error::InvalidBatchSizeCap);
    }
    Ok(())
}


// Retained for future batch-operation paths that enforce per-call caps.
#[allow(dead_code)]
pub(crate) fn validate_batch_len(batch_size: u32, cap: u32) -> Result<(), Error> {
    if batch_size == 0 || batch_size > cap {
        return Err(Error::InvalidBatchSize);
    }
    Ok(())
}


/// Returns the effective runtime cap for `batch_lock_funds`.
///
/// Returns the effective runtime cap for `batch_release_funds`.
pub(crate) fn get_max_release_batch_size(env: Env) -> u32 {
    get_batch_size_caps_internal(&env).release_cap
}


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

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


/// Return whether an escrow record is live, archived/restorable, or unknown.
pub fn probe_escrow_archival(env: Env, bounty_id: u64) -> PersistentRecordStatus {
    let marker = DataKey::EscrowTtl(bounty_id);
    let regular =
        persistent_record_status(&env, &marker, &DataKey::Escrow(bounty_id));
    if regular == PersistentRecordStatus::Missing {
        persistent_record_status(&env, &marker, &DataKey::EscrowAnon(bounty_id))
    } else {
        regular
    }
}


/// Return whether a pending claim is live, archived/restorable, or unknown.
pub fn probe_claim_archival(env: Env, bounty_id: u64) -> PersistentRecordStatus {
    persistent_record_status(
        &env,
        &DataKey::ClaimTtl(bounty_id),
        &DataKey::PendingClaim(bounty_id),
    )
}


/// Return whether a capability commitment is live, archived/restorable, or unknown.
pub fn probe_commitment_archival(
    env: Env,
    capability_id: BytesN<32>,
) -> PersistentRecordStatus {
    persistent_record_status(
        &env,
        &DataKey::CapabilityTtl(capability_id.clone()),
        &DataKey::Capability(capability_id),
    )
}


/// Return whether the global escrow index is live, archived/restorable, or unknown.
pub fn probe_index_archival(env: Env) -> PersistentRecordStatus {
    persistent_record_status(
        &env,
        &DataKey::EscrowIndexTtl,
        &DataKey::EscrowIndex,
    )
}


/// Return whether a depositor index is live, archived/restorable, or unknown.
pub fn probe_depositor_index_archival(
    env: Env,
    depositor: Address,
) -> PersistentRecordStatus {
    persistent_record_status(
        &env,
        &DataKey::DepositorIndexTtl(depositor.clone()),
        &DataKey::DepositorIndex(depositor),
    )
}


/// Initialize the contract with the admin address and the token address (XLM).
pub fn init(env: Env, admin: Address, token: Address) -> Result<(), Error> {
    if env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::AlreadyInitialized);
    }
    if admin == token {
        return Err(Error::Unauthorized);
    }
    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage().instance().set(&DataKey::Token, &token);
    // Version 2 reflects the breaking shared-trait interface alignment.
    env.storage().instance().set(&DataKey::Version, &2u32);
    env.storage().instance().set(
        &DataKey::RefundEligibilitySchemaVersion,
        &REFUND_ELIGIBILITY_SCHEMA_VERSION_V1,
    );
    // Upgrade-safe maintenance mode initialization (explicit key write).
    env.storage()
        .instance()
        .set(&DataKey::MaintenanceMode, &false);
    env.storage().instance().set(
        &DataKey::MaintenanceModeSchemaVersion,
        &MAINTENANCE_MODE_SCHEMA_VERSION_V1,
    );
    env.storage().instance().set(
        &DataKey::MaintenanceModeUpdatedAt,
        &env.ledger().timestamp(),
    );
    env.storage()
        .instance()
        .set(&DataKey::MaintenanceModeUpdatedBy, &admin);
    env.storage().instance().set(
        &DataKey::ParticipantListSchemaVersion,
        &PARTICIPANT_LIST_SCHEMA_VERSION_V1,
    );

    events::emit_bounty_initialized(
        &env,
        events::BountyEscrowInitialized {
            version: EVENT_VERSION_V2,
            admin: admin.clone(),
            token,
            timestamp: env.ledger().timestamp(),
        },
    );
    events::emit_fee_routing_schema_version_set(
        &env,
        events::FeeRoutingSchemaVersionSet {
            version: EVENT_VERSION_V2,
            schema_version: FEE_ROUTING_SCHEMA_VERSION_V1,
            set_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );
    // Emit audit event for maintenance mode schema version (upgrade-safe marker).
    events::emit_maintenance_mode_schema_version_set(
        &env,
        events::MaintenanceModeSchemaVersionSet {
            version: EVENT_VERSION_V2,
            schema_version: MAINTENANCE_MODE_SCHEMA_VERSION_V1,
            set_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );
    events::emit_participant_list_schema_version_set(
        &env,
        events::ParticipantListSchemaVersionSet {
            version: EVENT_VERSION_V2,
            schema_version: PARTICIPANT_LIST_SCHEMA_VERSION_V1,
            set_by: admin.clone(),
            timestamp: env.ledger().timestamp(),
        },
    );

    // Upgrade-safe high-value timelock config schema version initialization.
    env.storage().instance().set(
        &DataKey::HighValueConfigSchemaVersion,
        &HIGH_VALUE_CONFIG_SCHEMA_VERSION_V1,
    );
    events::emit_high_value_config_schema_version_set(
        &env,
        events::HighValueConfigSchemaVersionSet {
            version: EVENT_VERSION_V2,
            schema_version: HIGH_VALUE_CONFIG_SCHEMA_VERSION_V1,
            set_by: admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


pub fn init_with_network(
    env: Env,
    admin: Address,
    token: Address,
    chain_id: soroban_sdk::String,
    network_id: soroban_sdk::String,
) -> Result<(), Error> {
    init(env.clone(), admin, token)?;
    env.storage().instance().set(&DataKey::ChainId, &chain_id);
    env.storage()
        .instance()
        .set(&DataKey::NetworkId, &network_id);
    Ok(())
}


pub fn get_chain_id(env: Env) -> Option<soroban_sdk::String> {
    env.storage().instance().get(&DataKey::ChainId)
}


pub fn get_network_id(env: Env) -> Option<soroban_sdk::String> {
    env.storage().instance().get(&DataKey::NetworkId)
}


pub fn get_network_info(
    env: Env,
) -> (Option<soroban_sdk::String>, Option<soroban_sdk::String>) {
    (get_chain_id(env.clone()), get_network_id(env))
}


/// Return the persisted contract version.
pub fn get_version(env: Env) -> u32 {
    env.storage().instance().get(&DataKey::Version).unwrap_or(0)
}


/// Returns the currently active admin, or `None` if the contract is not initialized.
pub fn get_admin(env: Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}


/// Update the persisted contract version (admin only).
pub fn set_version(env: Env, new_version: u32) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();
    env.storage()
        .instance()
        .set(&DataKey::Version, &new_version);
    Ok(())
}


pub fn propose_admin(env: Env, new_admin: Address) {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));
    admin.require_auth();

    env.storage()
        .instance()
        .set(&DataKey::PendingAdmin, &new_admin);
    env.storage()
        .instance()
        .set(&DataKey::AdminTransferTimestamp, &env.ledger().timestamp());

    events::emit_admin_proposed(&env, admin, new_admin);
}


pub fn accept_admin(env: Env) {
    let pending: Address = env
        .storage()
        .instance()
        .get(&DataKey::PendingAdmin)
        .unwrap_or_else(|| panic!("No pending admin"));
    pending.require_auth();

    let start: u64 = env
        .storage()
        .instance()
        .get(&DataKey::AdminTransferTimestamp)
        .unwrap_or(0);
    let now = env.ledger().timestamp();

    if now < start + ADMIN_TIMELOCK {
        panic!("Timelock not expired");
    }

    let old_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));

    env.storage().instance().set(&DataKey::Admin, &pending);

    env.storage().instance().remove(&DataKey::PendingAdmin);
    env.storage()
        .instance()
        .remove(&DataKey::AdminTransferTimestamp);

    events::emit_admin_transferred(&env, old_admin, pending);
}


pub fn cancel_admin_transfer(env: Env) {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic!("Not initialized"));
    admin.require_auth();

    env.storage().instance().remove(&DataKey::PendingAdmin);
    env.storage()
        .instance()
        .remove(&DataKey::AdminTransferTimestamp);

    events::emit_admin_transfer_cancelled_v1(&env, admin);
}


/// Propose a new admin. The current admin remains active until the pending admin
/// explicitly accepts after the configured timelock.
pub fn propose_admin_rotation(env: Env, new_admin: Address) -> Result<u64, Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    if new_admin == admin {
        return Err(Error::InvalidAdminRotationTarget);
    }

    if env.storage().instance().has(&DataKey::PendingAdmin) {
        return Err(Error::AdminRotationAlreadyPending);
    }

    let timelock_duration = get_rotation_timelock_duration(env.clone());
    let timestamp = env.ledger().timestamp();
    let execute_after = timestamp.saturating_add(timelock_duration);

    env.storage()
        .instance()
        .set(&DataKey::PendingAdmin, &new_admin);
    env.storage()
        .instance()
        .set(&DataKey::AdminTimelock, &execute_after);

    emit_admin_rotation_proposed(
        &env,
        events::AdminRotationProposed {
            version: EVENT_VERSION_V2,
            current_admin: admin,
            pending_admin: new_admin,
            timelock_duration,
            execute_after,
            timestamp,
        },
    );

    Ok(execute_after)
}


/// Accept a previously proposed admin rotation once the timelock has elapsed.
pub fn accept_admin_rotation(env: Env) -> Result<Address, Error> {
    let pending_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::PendingAdmin)
        .ok_or(Error::AdminRotationNotPending)?;
    let execute_after: u64 = env
        .storage()
        .instance()
        .get(&DataKey::AdminTimelock)
        .ok_or(Error::AdminRotationNotPending)?;

    pending_admin.require_auth();

    let timestamp = env.ledger().timestamp();
    if timestamp < execute_after {
        return Err(Error::AdminRotationTimelockActive);
    }

    let previous_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;

    env.storage()
        .instance()
        .set(&DataKey::Admin, &pending_admin);
    env.storage().instance().remove(&DataKey::PendingAdmin);
    env.storage().instance().remove(&DataKey::AdminTimelock);

    emit_admin_rotation_accepted(
        &env,
        events::AdminRotationAccepted {
            version: EVENT_VERSION_V2,
            previous_admin,
            new_admin: pending_admin.clone(),
            timestamp,
        },
    );

    Ok(pending_admin)
}


/// Cancel a pending admin rotation while keeping the current admin unchanged.
pub fn cancel_admin_rotation(env: Env) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    let pending_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::PendingAdmin)
        .ok_or(Error::AdminRotationNotPending)?;

    env.storage().instance().remove(&DataKey::PendingAdmin);
    env.storage().instance().remove(&DataKey::AdminTimelock);

    emit_admin_rotation_cancelled(
        &env,
        events::AdminRotationCancelled {
            version: EVENT_VERSION_V2,
            admin,
            cancelled_pending_admin: pending_admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Update the global admin-rotation timelock duration.
pub fn set_rotation_timelock_duration(env: Env, duration: u64) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    if !(MIN_ADMIN_ROTATION_TIMELOCK..=MAX_ADMIN_ROTATION_TIMELOCK).contains(&duration) {
        return Err(Error::InvalidAdminRotationTimelock);
    }

    let previous_duration = get_rotation_timelock_duration(env.clone());
    env.storage()
        .instance()
        .set(&DataKey::TimelockDuration, &duration);

    emit_admin_rotation_timelock_updated(
        &env,
        events::AdminRotationTimelockUpdated {
            version: EVENT_VERSION_V2,
            admin,
            previous_duration,
            new_duration: duration,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Returns the configured timelock duration for future admin rotations.
pub fn get_rotation_timelock_duration(env: Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::TimelockDuration)
        .unwrap_or(DEFAULT_ADMIN_ROTATION_TIMELOCK)
}


/// Returns the pending admin, if a rotation is currently waiting for acceptance.
pub fn get_pending_admin(env: Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::PendingAdmin)
}


/// Returns the acceptance timestamp for the current pending admin rotation.
pub fn get_admin_rotation_timelock(env: Env) -> Option<u64> {
    env.storage().instance().get(&DataKey::AdminTimelock)
}


/// Returns comprehensive admin rotation state for indexing and UI display.
///
/// # Returns
/// - `Some(AdminRotationStatus)` if a rotation is pending
/// - `None` if no rotation is in progress
pub fn get_admin_rotation_status(env: Env) -> Option<AdminRotationStatus> {
    let pending_admin: Address = env.storage().instance().get(&DataKey::PendingAdmin)?;
    let execute_after: u64 = env.storage().instance().get(&DataKey::AdminTimelock)?;
    let current_admin: Address = env.storage().instance().get(&DataKey::Admin)?;
    let now = env.ledger().timestamp();

    Some(AdminRotationStatus {
        current_admin,
        pending_admin,
        execute_after,
        is_executable: now >= execute_after,
        remaining_seconds: if now < execute_after {
            execute_after.saturating_sub(now)
        } else {
            0
        },
        timestamp: now,
    })
}


/// Returns the full admin rotation configuration.
pub fn get_admin_rotation_config(env: Env) -> AdminRotationConfig {
    let duration = get_rotation_timelock_duration(env.clone());
    let has_pending = env.storage().instance().has(&DataKey::PendingAdmin);

    AdminRotationConfig {
        timelock_duration: duration,
        min_timelock: MIN_ADMIN_ROTATION_TIMELOCK,
        max_timelock: MAX_ADMIN_ROTATION_TIMELOCK,
        has_pending_rotation: has_pending,
        timestamp: env.ledger().timestamp(),
    }
}


/// Returns the effective max batch size for lock operations.
pub fn get_max_batch_size(env: Env) -> u32 {
    get_batch_size_caps_internal(&env).lock_cap
}


/// View: returns the effective batch size caps for lock and release operations.
///
/// When no caps have been configured by the admin, returns the compile-time
/// hard limit (`MAX_BATCH_SIZE`) for both fields.
pub fn get_batch_size_caps(env: Env) -> BatchSizeCaps {
    get_batch_size_caps_internal(&env)
}


/// Admin: configure independent batch size caps for lock and release operations.
///
/// Both caps must satisfy `1 <= cap <= MAX_BATCH_SIZE` (currently 20).
/// Setting a cap lower than the hard limit lets operators reduce the maximum
/// gas footprint of a single batch call without redeploying the contract.
///
/// # Errors
/// * `NotInitialized`     — contract not yet initialised
/// * `InvalidBatchSizeCap` — either cap is 0 or exceeds `MAX_BATCH_SIZE`
///
/// # Events
/// Emits [`events::BatchSizeCapsUpdated`] with previous and new values.
pub fn set_batch_size_caps(env: Env, lock_cap: u32, release_cap: u32) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let new_caps = BatchSizeCaps {
        lock_cap,
        release_cap,
    };
    validate_batch_size_caps(&new_caps)?;

    let previous = get_batch_size_caps_internal(&env);

    env.storage()
        .instance()
        .set(&DataKey::BatchSizeCaps, &new_caps);

    events::emit_batch_size_caps_updated(
        &env,
        events::BatchSizeCapsUpdated {
            version: EVENT_VERSION_V2,
            previous_lock_cap: previous.lock_cap,
            new_lock_cap: lock_cap,
            previous_release_cap: previous.release_cap,
            new_release_cap: release_cap,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Returns the notification preference bitmask for a bounty.
///
/// # Errors
/// * `BountyNotFound` — metadata for `bounty_id` does not exist.
pub fn get_notification_preferences(env: Env, bounty_id: u64) -> Result<u32, Error> {
    let metadata = env
        .storage()
        .persistent()
        .get::<DataKey, EscrowMetadata>(&DataKey::Metadata(bounty_id))
        .ok_or(Error::BountyNotFound)?;
    Ok(metadata.notification_prefs)
}


/// Sets the notification preference bitmask for a bounty (admin only).
///
/// `notification_prefs` is a bitfield combining [`NOTIFY_ON_LOCK`],
/// [`NOTIFY_ON_RELEASE`], [`NOTIFY_ON_DISPUTE`], and
/// [`NOTIFY_ON_EXPIRATION`]. Bits outside this mask are rejected with
/// [`Error::Unauthorized`].
///
/// # Events
/// Emits `NotificationPreferencesUpdated` with the previous and new
/// preference bitmasks.
///
/// # Errors
/// * `NotInitialized` — contract not yet initialised.
/// * `BountyNotFound` — metadata for `bounty_id` does not exist.
/// * `Unauthorized`  — caller is not admin, or `notification_prefs` contains reserved bits.
pub fn set_notification_preferences(
    env: Env,
    bounty_id: u64,
    notification_prefs: u32,
) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    if notification_prefs & !NOTIFICATION_PREFS_MASK != 0 {
        return Err(Error::Unauthorized);
    }

    let mut metadata = env
        .storage()
        .persistent()
        .get::<DataKey, EscrowMetadata>(&DataKey::Metadata(bounty_id))
        .ok_or(Error::BountyNotFound)?;

    let previous_prefs = metadata.notification_prefs;
    metadata.notification_prefs = notification_prefs;
    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &metadata);

    events::emit_notification_preferences_updated(
        &env,
        events::NotificationPreferencesUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_prefs,
            new_prefs: notification_prefs,
            actor: admin.clone(),
            created: false,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}

