//! Risk flags and escrow metadata: set, clear, update, and schema version.



use soroban_sdk::{Address, Env, String};
use crate::{
    events, rbac,
    DataKey, Error, EscrowMetadata,
    NOTIFICATION_PREFS_MASK, RISK_FLAG_MASK_ALL, RISK_FLAGS_VALID_MASK,
    events::{RiskFlagsUpdated, EVENT_VERSION_V2},
};

// ─────────────────────────────────────────────────────────────────
// Public entry points (dispatcher targets)
// ─────────────────────────────────────────────────────────────────


// =========================================================================
// RISK FLAGS GOVERNANCE
// =========================================================================

/// Set (OR-in) risk flag bits on a bounty's metadata (admin only).
///
/// # Invariants
/// - Only bits within [`RISK_FLAGS_VALID_MASK`] are accepted; any reserved
///   bits cause `InvalidRiskFlags`.
/// - Emits [`RiskFlagsUpdated`] after the new value is persisted (CEI).
/// - Metadata is created with all-zero flags if it does not yet exist.
///
/// # Security
/// - Admin-only; `require_auth` is called on the stored admin address.
/// - Flags are informational on-chain; enforcement belongs to off-chain services.
pub fn set_escrow_risk_flags(
    env: Env,
    bounty_id: u64,
    flags: u32,
) -> Result<EscrowMetadata, Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // Reject reserved bits.
    if flags & !RISK_FLAGS_VALID_MASK != 0 {
        return Err(Error::Unauthorized);
    }

    let mut meta: EscrowMetadata = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(&env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        });

    let previous_flags = meta.risk_flags;
    meta.risk_flags |= flags;

    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &meta);

    // Emit audit event after storage write (CEI ordering).
    emit_risk_flags_updated(
        &env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags: meta.risk_flags,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(meta)
}


/// Clear (AND-NOT) risk flag bits on a bounty's metadata (admin only).
///
/// # Invariants
/// - Only bits within [`RISK_FLAGS_VALID_MASK`] are accepted; any reserved
///   bits cause `InvalidRiskFlags`.
/// - Emits [`RiskFlagsUpdated`] after the new value is persisted (CEI).
/// - Idempotent: clearing already-cleared bits is a no-op (no error).
///
/// # Security
/// - Admin-only; `require_auth` is called on the stored admin address.
pub fn clear_escrow_risk_flags(
    env: Env,
    bounty_id: u64,
    flags: u32,
) -> Result<EscrowMetadata, Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    // Reject reserved bits.
    if flags & !RISK_FLAGS_VALID_MASK != 0 {
        return Err(Error::Unauthorized);
    }

    let mut meta: EscrowMetadata = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(&env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        });

    let previous_flags = meta.risk_flags;
    meta.risk_flags &= !flags;

    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &meta);

    // Emit audit event after storage write (CEI ordering).
    emit_risk_flags_updated(
        &env,
        RiskFlagsUpdated {
            version: EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags: meta.risk_flags,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(meta)
}


/// Get the metadata for a bounty. Returns a default (all-zero) record if
/// no metadata has been written yet.
///
/// Anonymization-aware: `EscrowMetadata` has no depositor-identifying field,
/// so this is safe to call for anonymously-locked bounties before resolution.
/// See `docs/anonymous-lock-privacy.md`.
pub fn get_metadata(env: Env, bounty_id: u64) -> EscrowMetadata {
    env.storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(&env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        })
}


/// Update the metadata fields for a bounty (admin only).
///
/// Risk flags are preserved from the existing record; use
/// `set_escrow_risk_flags` / `clear_escrow_risk_flags` to modify them.
pub fn update_metadata(
    env: Env,
    _admin: Address,
    bounty_id: u64,
    repo_id: u64,
    issue_id: u64,
    bounty_type: soroban_sdk::String,
    reference_hash: Option<soroban_sdk::Bytes>,
) -> Result<EscrowMetadata, Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }
    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    let existing: EscrowMetadata = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(&env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        });

    let updated = EscrowMetadata {
        repo_id,
        issue_id,
        bounty_type,
        risk_flags: existing.risk_flags, // preserve flags
        notification_prefs: existing.notification_prefs,
        reference_hash,
    };

    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &updated);

    Ok(updated)
}


/// Return the risk-flags governance storage schema version written during `init`.
/// Returns `0` on legacy deployments where the marker was never written.
pub fn get_risk_flags_schema_version(env: Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::RefundEligibilitySchemaVersion)
        .unwrap_or(0u32)
}


// ============================================================================
// RISK FLAGS GOVERNANCE
// ============================================================================

/// Updates the risk flags associated with a specific bounty.
///
/// # Access Control
/// Admin-only.
///
/// # Arguments
/// * `env` - The contract environment.
/// * `bounty_id` - The bounty identifier.
/// * `new_flags` - The new bitmask of risk flags to apply.
pub fn update_risk_flags(env: Env, bounty_id: u64, new_flags: u32) -> Result<(), Error> {
    if !env.storage().instance().has(&DataKey::Admin) {
        return Err(Error::NotInitialized);
    }

    let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
    admin.require_auth();

    if new_flags & !RISK_FLAG_MASK_ALL != 0 {
        return Err(Error::Unauthorized);
    }

    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    let mut metadata: EscrowMetadata = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id))
        .unwrap_or(EscrowMetadata {
            repo_id: 0,
            issue_id: 0,
            bounty_type: soroban_sdk::String::from_str(&env, ""),
            risk_flags: 0,
            notification_prefs: 0,
            reference_hash: None,
        });

    let previous_flags = metadata.risk_flags;
    metadata.risk_flags = new_flags;

    env.storage()
        .persistent()
        .set(&DataKey::Metadata(bounty_id), &metadata);

    events::emit_risk_flags_updated(
        &env,
        events::RiskFlagsUpdated {
            version: events::EVENT_VERSION_V2,
            bounty_id,
            previous_flags,
            new_flags,
            admin,
            timestamp: env.ledger().timestamp(),
        },
    );

    Ok(())
}


/// Retrieves the current risk flags for a given bounty.
pub fn get_risk_flags(env: Env, bounty_id: u64) -> Result<u32, Error> {
    if !env.storage().persistent().has(&DataKey::Escrow(bounty_id))
        && !env
            .storage()
            .persistent()
            .has(&DataKey::EscrowAnon(bounty_id))
    {
        return Err(Error::BountyNotFound);
    }

    let metadata: Option<EscrowMetadata> = env
        .storage()
        .persistent()
        .get(&DataKey::Metadata(bounty_id));

    Ok(metadata.map(|m| m.risk_flags).unwrap_or(0))
}

